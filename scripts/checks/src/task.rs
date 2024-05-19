use std::{collections::BTreeSet, fmt::Display, path::PathBuf};

use colored::Colorize;
use duct::cmd;
use itertools::Itertools;
use nix::{sys::signal::Signal, unistd::Pid};
use serde::{Serialize, Serializer};
use sha2::Digest;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::md_check;

#[derive(Debug, Clone, serde::Serialize, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SwayArtifacts {
    TypePaths,
    Normal,
}

#[derive(Debug, Default, Clone, serde::Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RustDeps {
    pub nightly: bool,
    #[serde(serialize_with = "comma_separated")]
    pub components: BTreeSet<String>,
}

fn comma_separated<S>(components: &BTreeSet<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let components = components.iter().join(",");
    components.serialize(serializer)
}

#[derive(Debug, Default, Clone, serde::Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CargoDeps {
    pub hack: bool,
    pub nextest: bool,
    pub machete: bool,
    pub udeps: bool,
}

impl std::ops::Add for CargoDeps {
    type Output = Self;
    fn add(mut self, other: Self) -> Self {
        self += other;
        self
    }
}

impl std::ops::AddAssign for CargoDeps {
    fn add_assign(&mut self, other: Self) {
        self.hack |= other.hack;
        self.nextest |= other.nextest;
        self.machete |= other.machete;
    }
}

#[derive(Debug, Default, Clone, serde::Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CiDeps {
    pub fuel_core_binary: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust: Option<RustDeps>,
    pub wasm: bool,
    pub cargo: CargoDeps,
    pub typos_cli: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sway_artifacts: Option<SwayArtifacts>,
}

impl std::ops::Add for CiDeps {
    type Output = Self;
    fn add(mut self, other: Self) -> Self {
        self += other;
        self
    }
}

impl std::ops::AddAssign for CiDeps {
    fn add_assign(&mut self, other: Self) {
        self.fuel_core_binary |= other.fuel_core_binary;

        let rust = match (self.rust.take(), other.rust) {
            (Some(mut self_rust), Some(other_rust)) => {
                self_rust.nightly |= other_rust.nightly;
                self_rust.components = self_rust
                    .components
                    .union(&other_rust.components)
                    .cloned()
                    .collect();
                Some(self_rust)
            }
            (Some(self_rust), None) => Some(self_rust),
            (None, Some(other_rust)) => Some(other_rust),
            (None, None) => None,
        };
        self.rust = rust;

        self.wasm |= other.wasm;
        self.cargo += other.cargo;
        self.typos_cli |= other.typos_cli;

        let sway_artifacts = match (self.sway_artifacts, other.sway_artifacts) {
            (Some(self_sway), Some(other_sway)) => {
                if self_sway != other_sway {
                    panic!(
                        "Deps cannot be unified. Cannot have type paths and normal artifacts at once! {self_sway:?} != {other_sway:?}",
                    );
                }
                Some(self_sway)
            }
            (Some(self_sway), None) => Some(self_sway),
            (None, Some(other_sway)) => Some(other_sway),
            (None, None) => None,
        };
        self.sway_artifacts = sway_artifacts;
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CiJob {
    deps: CiDeps,
    #[serde(serialize_with = "serialize_as_ids")]
    tasks: Vec<Task>,
}

fn serialize_as_ids<S>(tasks: &[Task], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ids = tasks.iter().map(|task| task.id()).join(",");
    ids.serialize(serializer)
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Task {
    pub cwd: PathBuf,
    pub cmd: Command,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Command {
    Custom {
        program: String,
        args: Vec<String>,
        env: Vec<(String, String)>,
        deps: CiDeps,
    },
    MdCheck,
}

impl Command {
    pub fn deps(&self) -> CiDeps {
        match self {
            Command::Custom { deps, .. } => deps.clone(),
            Command::MdCheck => CiDeps::default(),
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
    pub tasks: BTreeSet<Task>,
}

impl Tasks {
    pub fn ci_jobs(&self) -> Vec<CiJob> {
        self.tasks
            .iter()
            .sorted_by_key(|task| task.cwd.clone())
            .group_by(|task| task.cwd.clone())
            .into_iter()
            .flat_map(|(_, tasks)| {
                let (tasks_requiring_type_paths, normal_tasks): (Vec<_>, Vec<_>) =
                    tasks.into_iter().partition(|task| {
                        task.cmd
                            .deps()
                            .sway_artifacts
                            .is_some_and(|dep| matches!(dep, SwayArtifacts::TypePaths))
                    });

                let type_paths_deps = tasks_requiring_type_paths
                    .iter()
                    .map(|ty| ty.cmd.deps())
                    .reduce(|acc, next| acc + next);

                let normal_deps = normal_tasks
                    .iter()
                    .map(|ty| ty.cmd.deps())
                    .reduce(|acc, next| acc + next);

                let mut jobs = vec![];

                if let Some(deps) = type_paths_deps {
                    jobs.push(CiJob {
                        deps,
                        tasks: tasks_requiring_type_paths.into_iter().cloned().collect(),
                    });
                }

                if let Some(deps) = normal_deps {
                    jobs.push(CiJob {
                        deps,
                        tasks: normal_tasks.into_iter().cloned().collect(),
                    });
                }

                jobs
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
