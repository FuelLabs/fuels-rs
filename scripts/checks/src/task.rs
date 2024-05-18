use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fmt::Display,
    path::{Path, PathBuf},
};

use colored::Colorize;
use duct::cmd;
use itertools::Itertools;
use nix::{sys::signal::Signal, unistd::Pid};
use sha2::Digest;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::md_check;

#[derive(Debug, Clone, serde::Serialize, Copy)]
pub enum SwayProjectDep {
    TypePaths,
    Normal,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CiDeps {
    fuel_core_binary: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    rust_toolchain_components: Option<String>,
    wasm: bool,
    cargo_hack: bool,
    nextest: bool,
    cargo_machete: bool,
    typos_cli: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    sway_project_artifacts: Option<SwayProjectDep>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CiJob {
    dir: PathBuf,
    deps: CiDeps,
}

impl CiDeps {
    pub fn from_deps(deps: &HashSet<Dependency>) -> Self {
        let fuel_core_binary = deps.contains(&Dependency::FuelCoreBinary);
        let wasm = deps.contains(&Dependency::Wasm);
        let cargo_hack = deps.contains(&Dependency::CargoHack);
        let nextest = deps.contains(&Dependency::Nextest);
        let cargo_machete = deps.contains(&Dependency::CargoMachete);
        let typos_cli = deps.contains(&Dependency::TyposCli);

        let sway_project_artifacts = deps.iter().find_map(|dep| match dep {
            Dependency::SwayArtifacts { type_paths } if *type_paths => {
                Some(SwayProjectDep::TypePaths)
            }
            Dependency::SwayArtifacts { .. } => Some(SwayProjectDep::Normal),
            _ => None,
        });

        let mut components = String::new();
        if deps.contains(&Dependency::Clippy) {
            components.push_str("clippy");
        }
        if deps.contains(&Dependency::RustFmt) {
            if !components.is_empty() {
                components.push(',');
            }
            components.push_str("rustfmt");
        }

        let rust_toolchain_components = if components.is_empty() {
            None
        } else {
            Some(components)
        };

        Self {
            fuel_core_binary,
            rust_toolchain_components,
            wasm,
            cargo_hack,
            nextest,
            cargo_machete,
            typos_cli,
            sway_project_artifacts,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Task {
    pub cwd: PathBuf,
    pub cmd: Command,
}

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Dependency {
    FuelCoreBinary,
    Clippy,
    CargoHack,
    RustFmt,
    RustStable,
    RustNightly,
    SwayArtifacts { type_paths: bool },
    Nextest,
    CargoMachete,
    Wasm,
    TyposCli,
    Grep,
    Find,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Command {
    Custom {
        program: String,
        args: Vec<String>,
        env: Vec<(String, String)>,
        deps: Vec<Dependency>,
    },
    MdCheck,
}

impl Command {
    pub fn deps(&self) -> HashSet<Dependency> {
        match self {
            Command::Custom { deps, .. } => HashSet::from_iter(deps.iter().copied()),
            Command::MdCheck => HashSet::from_iter([Dependency::Grep, Dependency::Find]),
        }
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Custom {
                program, args, env, ..
            } => {
                let args = args.iter().join(" ");
                if env.is_empty() {
                    write!(f, "{program} {args}")
                } else {
                    let env = env
                        .iter()
                        .map(|(key, value)| format!("{key}='{value}'"))
                        .join(" ");
                    write!(f, "{env} {program} {args}")
                }
            }
            Command::MdCheck { .. } => write!(f, "MdCheck"),
        }
    }
}

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Task {}, dir: {:?}, {}", self.id(), self.cwd, self.cmd)
    }
}

#[derive(Debug, Clone)]
pub struct Execution {
    pub cmd_desc: String,
    pub status: ExecutionStatus,
}

impl From<std::io::Error> for ExecutionStatus {
    fn from(value: std::io::Error) -> Self {
        Self::Failed {
            reason: value.to_string(),
        }
    }
}

impl From<anyhow::Error> for ExecutionStatus {
    fn from(value: anyhow::Error) -> Self {
        Self::Failed {
            reason: value.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExecutionStatus {
    Success { out: String },
    Failed { reason: String },
}

impl Execution {
    pub fn report(&self, tty: bool, verbose: bool) -> String {
        let status = match &self.status {
            ExecutionStatus::Failed { reason } => {
                let err = if tty { "error".red() } else { "error".normal() };
                format!("{err}\n{reason}")
            }
            ExecutionStatus::Success { out } => {
                let ok = if tty { "ok".green() } else { "ok".normal() };
                if verbose {
                    format!("{ok}\n{out}")
                } else {
                    ok.to_string()
                }
            }
        };

        format!("{} ... {status}", self.cmd_desc)
    }
}

impl Task {
    pub fn id(&self) -> String {
        let mut hasher = sha2::Sha256::default();
        hasher.update(format!("{:?}", self).as_bytes());
        hex::encode(&hasher.finalize()[..8])
    }

    pub fn run(self) -> Execution {
        match &self.cmd {
            Command::Custom {
                program, args, env, ..
            } => self.run_custom(program, args.iter().map(|e| e.as_str()), env),
            Command::MdCheck => self.run_md_check(),
        }
    }

    fn run_md_check(&self) -> Execution {
        let status = if let Err(e) = md_check::run(&self.cwd) {
            e.into()
        } else {
            ExecutionStatus::Success {
                out: "".to_string(),
            }
        };

        self.execution_report(status)
    }

    fn run_custom<'a, F>(&self, program: &str, args: F, env: &[(String, String)]) -> Execution
    where
        F: IntoIterator<Item = &'a str>,
    {
        let mut cmd = cmd(program, args)
            .stderr_to_stdout()
            .dir(&self.cwd)
            .stdin_null()
            .stdout_capture()
            .unchecked();

        for (key, value) in env {
            cmd = cmd.env(key, value);
        }

        let output = match cmd.run() {
            Ok(output) => output,
            Err(err) => return self.execution_report(err),
        };

        let decoded = String::from_utf8_lossy(&output.stdout).into_owned();
        let status = if output.status.success() {
            ExecutionStatus::Success { out: decoded }
        } else {
            ExecutionStatus::Failed { reason: decoded }
        };

        self.execution_report(status)
    }

    fn execution_report(&self, status: impl Into<ExecutionStatus>) -> Execution {
        Execution {
            cmd_desc: self.to_string(),
            status: status.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tasks {
    pub tasks: Vec<Task>,
}

impl Tasks {
    pub fn ci_jobs(&self) -> Vec<CiJob> {
        self.tasks
            .iter()
            .group_by(|task| &task.cwd)
            .into_iter()
            .map(|(cwd, tasks)| {
                let deps = tasks.flat_map(|task| task.cmd.deps()).collect();
                CiJob {
                    dir: cwd.clone(),
                    deps: CiDeps::from_deps(&deps),
                }
            })
            .collect()
    }

    pub fn verify_no_duplicates(&self) -> anyhow::Result<()> {
        let duplicates = self
            .tasks
            .iter()
            .duplicates_by(|task| task.id())
            .collect_vec();

        if !duplicates.is_empty() {
            anyhow::bail!("Found duplicate tasks: {:#?}", duplicates);
        }

        Ok(())
    }

    pub async fn run(
        self,
        tty: bool,
        verbose: bool,
        cancel_token: CancellationToken,
    ) -> anyhow::Result<()> {
        self.verify_no_duplicates()?;
        let mut set = JoinSet::new();
        for task in self.tasks {
            set.spawn_blocking(|| task.run());
        }

        let mut errors = false;

        let mut handle_task_response = |execution: Execution| {
            if let ExecutionStatus::Failed { .. } = execution.status {
                errors = true;
            }

            let report = execution.report(tty, verbose);
            eprintln!("{report}");
            anyhow::Ok(())
        };

        let kill_processes = || {
            // All spawned processes are in the same process group created in main.
            nix::sys::signal::killpg(Pid::from_raw(0), Signal::SIGINT)
        };

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    kill_processes()?;
                    return Ok(());
                }
                task_response = set.join_next() => {
                    if let Some(result) = task_response {
                        handle_task_response(result?)?;
                    } else {
                        break;
                    }
                }
            }
        }

        if errors {
            anyhow::bail!("Some checks failed");
        }

        Ok(())
    }

    pub fn retain_with_ids(&mut self, ids: &[String]) {
        self.tasks.retain(|task| ids.contains(&task.id()));
    }

    pub fn retain_with_dirs(&mut self, dirs: &[String]) {
        self.tasks.retain(|task| {
            dirs.contains(
                &task
                    .cwd
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .to_string(),
            )
        });
    }
}

#[cfg(test)]
mod tests {
    //
    // use std::collections::HashMap;
    //
    // use super::*;
    // use pretty_assertions::assert_eq;
    //
    // #[test]
    // fn selection_respected() {
    //     // given
    //     let config = vec![
    //         config::TasksDescription {
    //             run_for_dirs: vec![PathBuf::from("some/foo"), PathBuf::from("other/zoo")],
    //             commands: vec![
    //                 config::Command::Custom {
    //                     cmd: vec!["cargo".to_string(), "check".to_string()],
    //                     env: None,
    //                     run_if: None,
    //                 },
    //                 config::Command::Custom {
    //                     cmd: vec!["cargo".to_string(), "fmt".to_string()],
    //                     env: None,
    //                     run_if: None,
    //                 },
    //                 config::Command::Custom {
    //                     cmd: vec!["cargo".to_string(), "test".to_string()],
    //                     env: None,
    //                     run_if: None,
    //                 },
    //             ],
    //         },
    //         config::TasksDescription {
    //             run_for_dirs: vec![PathBuf::from("some/boo")],
    //             commands: vec![
    //                 config::Command::Custom {
    //                     cmd: vec!["cargo".to_string(), "check".to_string()],
    //                     env: None,
    //                     run_if: None,
    //                 },
    //                 config::Command::Custom {
    //                     cmd: vec!["cargo".to_string(), "fmt".to_string()],
    //                     env: None,
    //                     run_if: None,
    //                 },
    //                 config::Command::Custom {
    //                     cmd: vec!["cargo".to_string(), "test".to_string()],
    //                     env: None,
    //                     run_if: None,
    //                 },
    //             ],
    //         },
    //     ];
    //
    //     let mut tasks = Tasks::from_task_descriptions(config, ".");
    //
    //     use rand::seq::SliceRandom;
    //     let random_task = tasks.tasks.choose(&mut rand::thread_rng()).unwrap().clone();
    //
    //     // when
    //     tasks.retain_with_ids(&[random_task.id()]);
    //
    //     // then
    //     assert_eq!(tasks.tasks, [random_task]);
    // }
    //
    // #[test]
    // fn workspace_root_respected() {
    //     // given
    //     let config = vec![config::TasksDescription {
    //         run_for_dirs: vec![PathBuf::from("some/foo")],
    //         commands: vec![config::Command::Custom {
    //             cmd: vec!["cargo".to_string(), "check".to_string()],
    //             env: None,
    //             run_if: None,
    //         }],
    //     }];
    //
    //     // when
    //     let mut tasks = Tasks::from_task_descriptions(config, "workspace");
    //
    //     // then
    //     let mut expected = [Task {
    //         cwd: PathBuf::from("workspace/some/foo"),
    //         cmd: Command::Custom {
    //             program: "cargo".to_string(),
    //             args: vec!["check".to_string()],
    //             env: vec![],
    //         },
    //     }];
    //
    //     expected.sort();
    //     tasks.tasks.sort();
    //     assert_eq!(tasks.tasks, expected);
    // }
    //
    // #[test]
    // fn ignore_if_in_dir_respected() {
    //     // given
    //     let config = vec![config::TasksDescription {
    //         run_for_dirs: vec![PathBuf::from("boom/some/foo")],
    //         commands: vec![config::Command::Custom {
    //             cmd: vec!["cargo".to_string(), "check".to_string()],
    //             env: None,
    //             run_if: Some(RunIf::CwdDoesntEndWith(vec!["some/foo".to_string()])),
    //         }],
    //     }];
    //
    //     // when
    //     let tasks = Tasks::from_task_descriptions(config, ".");
    //
    //     // then
    //     assert_eq!(tasks.tasks, []);
    // }
}
