use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use colored::Colorize;
use duct::cmd;
use itertools::Itertools;
use sha2::Digest;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::{
    config::{self, RunIf, TasksDescription},
    md_check,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Task {
    pub cwd: PathBuf,
    pub cmd: Action,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Action {
    Custom {
        program: String,
        args: Vec<String>,
        env: Vec<(String, String)>,
    },
    MdCheck,
}

pub struct Command {
    run_if: Option<RunIf>,
    action: Action,
}

impl From<config::Command> for Command {
    fn from(value: config::Command) -> Self {
        match value {
            config::Command::Custom {
                cmd: parts,
                env,
                run_if,
            } => {
                let program = parts.first().unwrap().to_string();
                let args = parts.into_iter().skip(1).map(|s| s.to_string()).collect();
                Self {
                    run_if,
                    action: Action::Custom {
                        program,
                        args,
                        env: env.unwrap_or_default().into_iter().collect(),
                    },
                }
            }
            config::Command::MdCheck { run_if } => Self {
                action: Action::MdCheck,
                run_if,
            },
        }
    }
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Custom { program, args, env } => {
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
            Action::MdCheck { .. } => write!(f, "MdCheck"),
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
            Action::Custom { program, args, env } => {
                self.run_custom(program, args.iter().map(|e| e.as_str()), env)
            }
            Action::MdCheck => self.run_md_check(),
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
    pub fn used_dirs(&self) -> Vec<PathBuf> {
        self.tasks
            .iter()
            .map(|task| &task.cwd)
            .unique()
            .cloned()
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

    pub fn from_task_descriptions(
        groups: Vec<TasksDescription>,
        workspace_root: impl AsRef<Path>,
    ) -> Self {
        let workspace_root = workspace_root.as_ref();
        let tasks = groups
            .into_iter()
            .flat_map(|entry| {
                entry.run_for_dirs.into_iter().flat_map(move |dir| {
                    let cwd = workspace_root.join(dir);
                    entry
                        .commands
                        .iter()
                        .cloned()
                        .map(Command::from)
                        .filter(|cmd| {
                            if let Some(condition) = &cmd.run_if {
                                condition.should_run(&cwd)
                            } else {
                                true
                            }
                        })
                        .map(|cmd| Task {
                            cwd: cwd.clone(),
                            cmd: cmd.action,
                        })
                        .collect_vec()
                })
            })
            .collect();

        Self { tasks }
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
        tokio::select! {
            _ = cancel_token.cancelled() => {
            return Ok(());
            }
            else => {

            }
        }
        while let Some(result) = set.join_next().await {
            let execution = result?;
            if let ExecutionStatus::Success { .. } = execution.status {
                errors = true;
            }

            let report = execution.report(tty, verbose);
            eprintln!("{report}");
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

    use std::collections::HashMap;

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn tasks_correctly_generated() {
        // given
        let first = config::TasksDescription {
            run_for_dirs: vec![PathBuf::from("some/foo1"), PathBuf::from("some/foo2")],
            commands: vec![
                config::Command::Custom {
                    cmd: vec!["cargo".to_string(), "check".to_string()],
                    env: Some(HashMap::from_iter(vec![(
                        "FOO".to_string(),
                        "BAR".to_string(),
                    )])),
                    run_if: None,
                },
                config::Command::MdCheck { run_if: None },
            ],
        };
        let second = config::TasksDescription {
            run_for_dirs: vec![PathBuf::from("some/boo")],
            commands: vec![
                config::Command::Custom {
                    cmd: vec!["cargo".to_string(), "test".to_string()],
                    env: None,
                    run_if: None,
                },
                config::Command::Custom {
                    cmd: vec!["cargo".to_string(), "fmt".to_string()],
                    env: None,
                    run_if: None,
                },
            ],
        };

        // when
        let mut tasks = Tasks::from_task_descriptions(vec![first, second], ".");

        // then
        let group1_path1 = PathBuf::from("./some/foo1");
        let group1_path2 = PathBuf::from("./some/foo2");
        let group2_path1 = PathBuf::from("./some/boo");

        tasks.tasks.sort();
        let mut expected = [
            Task {
                cwd: group1_path1.clone(),
                cmd: Action::Custom {
                    program: "cargo".to_string(),
                    args: vec!["check".to_string()],
                    env: vec![("FOO".to_string(), "BAR".to_string())],
                },
            },
            Task {
                cwd: group1_path1.clone(),
                cmd: Action::MdCheck,
            },
            Task {
                cwd: group1_path2.clone(),
                cmd: Action::Custom {
                    program: "cargo".to_string(),
                    args: vec!["check".to_string()],
                    env: vec![("FOO".to_string(), "BAR".to_string())],
                },
            },
            Task {
                cwd: group1_path2.clone(),
                cmd: Action::MdCheck,
            },
            Task {
                cwd: group2_path1.clone(),
                cmd: Action::Custom {
                    program: "cargo".to_string(),
                    args: vec!["fmt".to_string()],
                    env: vec![],
                },
            },
            Task {
                cwd: group2_path1.clone(),
                cmd: Action::Custom {
                    program: "cargo".to_string(),
                    args: vec!["test".to_string()],
                    env: vec![],
                },
            },
        ];

        expected.sort();
        assert_eq!(tasks.tasks, expected);
    }

    #[test]
    fn selection_respected() {
        // given
        let config = vec![
            config::TasksDescription {
                run_for_dirs: vec![PathBuf::from("some/foo"), PathBuf::from("other/zoo")],
                commands: vec![
                    config::Command::Custom {
                        cmd: vec!["cargo".to_string(), "check".to_string()],
                        env: None,
                        run_if: None,
                    },
                    config::Command::Custom {
                        cmd: vec!["cargo".to_string(), "fmt".to_string()],
                        env: None,
                        run_if: None,
                    },
                    config::Command::Custom {
                        cmd: vec!["cargo".to_string(), "test".to_string()],
                        env: None,
                        run_if: None,
                    },
                ],
            },
            config::TasksDescription {
                run_for_dirs: vec![PathBuf::from("some/boo")],
                commands: vec![
                    config::Command::Custom {
                        cmd: vec!["cargo".to_string(), "check".to_string()],
                        env: None,
                        run_if: None,
                    },
                    config::Command::Custom {
                        cmd: vec!["cargo".to_string(), "fmt".to_string()],
                        env: None,
                        run_if: None,
                    },
                    config::Command::Custom {
                        cmd: vec!["cargo".to_string(), "test".to_string()],
                        env: None,
                        run_if: None,
                    },
                ],
            },
        ];

        let mut tasks = Tasks::from_task_descriptions(config, ".");

        // when
        tasks.retain_with_ids(&["zoo".to_string()]);

        // then
        let cwd = PathBuf::from("./other/zoo");

        let mut expected = [
            Task {
                cwd: cwd.clone(),
                cmd: Action::Custom {
                    program: "cargo".to_string(),
                    args: vec!["check".to_string()],
                    env: vec![],
                },
            },
            Task {
                cwd: cwd.clone(),
                cmd: Action::Custom {
                    program: "cargo".to_string(),
                    args: vec!["fmt".to_string()],
                    env: vec![],
                },
            },
            Task {
                cwd: cwd.clone(),
                cmd: Action::Custom {
                    program: "cargo".to_string(),
                    args: vec!["test".to_string()],
                    env: vec![],
                },
            },
        ];

        tasks.tasks.sort();
        expected.sort();
        assert_eq!(tasks.tasks, expected);
    }

    #[test]
    fn workspace_root_respected() {
        // given
        let config = vec![config::TasksDescription {
            run_for_dirs: vec![PathBuf::from("some/foo")],
            commands: vec![config::Command::Custom {
                cmd: vec!["cargo".to_string(), "check".to_string()],
                env: None,
                run_if: None,
            }],
        }];

        // when
        let mut tasks = Tasks::from_task_descriptions(config, "workspace");

        // then
        let mut expected = [Task {
            cwd: PathBuf::from("workspace/some/foo"),
            cmd: Action::Custom {
                program: "cargo".to_string(),
                args: vec!["check".to_string()],
                env: vec![],
            },
        }];

        expected.sort();
        tasks.tasks.sort();
        assert_eq!(tasks.tasks, expected);
    }

    #[test]
    fn ignore_if_in_dir_respected() {
        // given
        let config = vec![config::TasksDescription {
            run_for_dirs: vec![PathBuf::from("boom/some/foo")],
            commands: vec![config::Command::Custom {
                cmd: vec!["cargo".to_string(), "check".to_string()],
                env: None,
                run_if: Some(RunIf::CwdDoesntEndWith(vec!["some/foo".to_string()])),
            }],
        }];

        // when
        let tasks = Tasks::from_task_descriptions(config, ".");

        // then
        assert_eq!(tasks.tasks, []);
    }
}
