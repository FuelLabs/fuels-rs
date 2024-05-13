use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use colored::Colorize;
use duct::cmd;
use itertools::Itertools;
use tokio::task::JoinSet;

use crate::{config, md_check};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Task {
    cwd: PathBuf,
    name: String,
    cmd: Command,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Command {
    Custom { program: String, args: Vec<String> },
    MdCheck { ignore: Vec<PathBuf> },
}

impl From<config::Command> for Command {
    fn from(value: config::Command) -> Self {
        match value {
            config::Command::Custom(parts) => {
                let program = parts.first().unwrap().to_string();
                let args = parts.into_iter().skip(1).map(|s| s.to_string()).collect();
                Self::Custom { program, args }
            }
            config::Command::MdCheck { ignore } => Self::MdCheck { ignore },
        }
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Custom { program, args } => {
                let args = args.iter().join(" ");
                write!(f, "{program} {args}")
            }
            Command::MdCheck { .. } => write!(f, "MdCheck"),
        }
    }
}

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}) {}", self.name, self.cmd)
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
    pub fn run(self) -> Execution {
        match &self.cmd {
            Command::Custom { program, args } => self.run_custom(program, args),
            Command::MdCheck { ignore } => self.run_md_check(ignore),
        }
    }

    fn run_md_check(&self, ignore: &[PathBuf]) -> Execution {
        let ignore = ignore
            .iter()
            .map(|path| self.cwd.join(path))
            .collect::<Vec<_>>();
        let status = if let Err(e) = md_check::run(&self.cwd, ignore) {
            e.into()
        } else {
            ExecutionStatus::Success {
                out: "".to_string(),
            }
        };

        self.execution_report(status)
    }

    fn run_custom(&self, program: &String, args: &Vec<String>) -> Execution {
        let cmd = cmd(program, args)
            .stderr_to_stdout()
            .dir(&self.cwd)
            .stdin_null()
            .stdout_capture()
            .unchecked();

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
    pub fn from_config(config: config::Config, workspace_root: impl AsRef<Path>) -> Self {
        let workspace_root = workspace_root.as_ref();
        let tasks = config
            .0
            .into_iter()
            .flat_map(|entry| {
                let cwd = workspace_root.join(&entry.working_dir);
                let name = entry.name;
                entry
                    .commands
                    .into_iter()
                    .map(Command::from)
                    .map(move |cmd| Task {
                        cwd: cwd.clone(),
                        name: name.clone(),
                        cmd,
                    })
            })
            .collect();

        Self { tasks }
    }

    pub async fn run(self, tty: bool, verbose: bool) -> anyhow::Result<()> {
        let mut set = JoinSet::new();
        for task in self.tasks {
            set.spawn_blocking(|| task.run());
        }

        while let Some(result) = set.join_next().await {
            let result = result?;
            eprintln!("{}", result.report(tty, verbose));
        }

        Ok(())
    }

    pub fn retain(&mut self, crate_names: &[String]) {
        self.tasks.retain(|task| crate_names.contains(&task.name));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn tasks_correctly_generated() {
        // given
        let first = config::Group {
            working_dir: PathBuf::from("some/foo"),
            name: "foo".to_string(),
            commands: vec![
                config::Command::Custom(vec!["cargo".to_string(), "check".to_string()]),
                config::Command::Custom(vec!["cargo".to_string(), "check".to_string()]),
                config::Command::Custom(vec!["cargo".to_string(), "fmt".to_string()]),
                config::Command::Custom(vec!["cargo".to_string(), "test".to_string()]),
                config::Command::Custom(vec!["cargo".to_string(), "test".to_string()]),
            ],
        };
        let second = config::Group {
            working_dir: PathBuf::from("some/boo"),
            name: "boo".to_string(),
            commands: vec![
                config::Command::Custom(vec!["cargo".to_string(), "test".to_string()]),
                config::Command::Custom(vec!["cargo".to_string(), "fmt".to_string()]),
            ],
        };

        // when
        let mut tasks = Tasks::from_config(config::Config(vec![first, second]), ".");

        // then
        let crate1 = PathBuf::from("./some/foo");
        let crate2 = PathBuf::from("./some/boo");

        tasks.tasks.sort();
        let mut expected = [
            Task {
                name: "foo".to_string(),
                cwd: crate1.clone(),
                cmd: Command::Custom {
                    program: "cargo".to_string(),
                    args: vec!["check".to_string()],
                },
            },
            Task {
                name: "foo".to_string(),
                cwd: crate1.clone(),
                cmd: Command::Custom {
                    program: "cargo".to_string(),
                    args: vec!["check".to_string()],
                },
            },
            Task {
                name: "foo".to_string(),
                cwd: crate1.clone(),
                cmd: Command::Custom {
                    program: "cargo".to_string(),
                    args: vec!["fmt".to_string()],
                },
            },
            Task {
                name: "foo".to_string(),
                cwd: crate1.clone(),
                cmd: Command::Custom {
                    program: "cargo".to_string(),
                    args: vec!["test".to_string()],
                },
            },
            Task {
                name: "foo".to_string(),
                cwd: crate1.clone(),
                cmd: Command::Custom {
                    program: "cargo".to_string(),
                    args: vec!["test".to_string()],
                },
            },
            Task {
                name: "boo".to_string(),
                cwd: crate2.clone(),
                cmd: Command::Custom {
                    program: "cargo".to_string(),
                    args: vec!["fmt".to_string()],
                },
            },
            Task {
                name: "boo".to_string(),
                cwd: crate2.clone(),
                cmd: Command::Custom {
                    program: "cargo".to_string(),
                    args: vec!["test".to_string()],
                },
            },
        ];

        expected.sort();
        assert_eq!(tasks.tasks, expected);
    }

    #[test]
    fn selection_respected() {
        // given
        let config = config::Config(vec![
            config::Group {
                working_dir: PathBuf::from("some/foo"),
                name: "foo".to_string(),
                commands: vec![
                    config::Command::Custom(vec!["cargo".to_string(), "check".to_string()]),
                    config::Command::Custom(vec!["cargo".to_string(), "fmt".to_string()]),
                    config::Command::Custom(vec!["cargo".to_string(), "test".to_string()]),
                ],
            },
            config::Group {
                working_dir: PathBuf::from("some/boo"),
                name: "boo".to_string(),
                commands: vec![
                    config::Command::Custom(vec!["cargo".to_string(), "check".to_string()]),
                    config::Command::Custom(vec!["cargo".to_string(), "fmt".to_string()]),
                    config::Command::Custom(vec!["cargo".to_string(), "test".to_string()]),
                ],
            },
        ]);

        let mut tasks = Tasks::from_config(config, ".");

        // when
        tasks.retain(&["foo".to_string()]);

        // then
        let cwd = PathBuf::from("./some/foo");

        let mut expected = [
            Task {
                name: "foo".to_string(),
                cwd: cwd.clone(),
                cmd: Command::Custom {
                    program: "cargo".to_string(),
                    args: vec!["check".to_string()],
                },
            },
            Task {
                name: "foo".to_string(),
                cwd: cwd.clone(),
                cmd: Command::Custom {
                    program: "cargo".to_string(),
                    args: vec!["fmt".to_string()],
                },
            },
            Task {
                name: "foo".to_string(),
                cwd: cwd.clone(),
                cmd: Command::Custom {
                    program: "cargo".to_string(),
                    args: vec!["test".to_string()],
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
        let config = config::Config(vec![config::Group {
            name: "foo".to_string(),
            working_dir: PathBuf::from("some/foo"),
            commands: vec![config::Command::Custom(vec![
                "cargo".to_string(),
                "check".to_string(),
            ])],
        }]);

        // when
        let mut tasks = Tasks::from_config(config, "workspace");

        // then
        let mut expected = [Task {
            name: "foo".to_string(),
            cwd: PathBuf::from("workspace/some/foo"),
            cmd: Command::Custom {
                program: "cargo".to_string(),
                args: vec!["check".to_string()],
            },
        }];

        expected.sort();
        tasks.tasks.sort();
        assert_eq!(tasks.tasks, expected);
    }
}
