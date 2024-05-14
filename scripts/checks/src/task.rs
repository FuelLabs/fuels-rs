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
    cmd: Action,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Action {
    Custom {
        program: String,
        args: Vec<String>,
        env: Vec<(String, String)>,
    },
    MdCheck,
}

pub struct Command {
    ignore_if_cwd_ends_with: Vec<String>,
    action: Action,
}

impl From<config::Command> for Command {
    fn from(value: config::Command) -> Self {
        match value {
            config::Command::Custom {
                cmd: parts,
                ignore_if_cwd_ends_with,
                env,
            } => {
                let program = parts.first().unwrap().to_string();
                let args = parts.into_iter().skip(1).map(|s| s.to_string()).collect();
                Self {
                    ignore_if_cwd_ends_with: ignore_if_cwd_ends_with.unwrap_or_default(),
                    action: Action::Custom {
                        program,
                        args,
                        env: env.unwrap_or_default().into_iter().collect(),
                    },
                }
            }
            config::Command::MdCheck { ignore_if_in_dir } => Self {
                ignore_if_cwd_ends_with: ignore_if_in_dir.unwrap_or_default(),
                action: Action::MdCheck,
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
        write!(f, "({:?}) {}", self.cwd, self.cmd)
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
    pub fn from_config(config: config::Config, workspace_root: impl AsRef<Path>) -> Self {
        let workspace_root = workspace_root.as_ref();
        let tasks = config
            .0
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
                            cmd.ignore_if_cwd_ends_with
                                .iter()
                                .all(|ignored_dir| !cwd.ends_with(ignored_dir))
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

    pub async fn run(self, tty: bool, verbose: bool) -> anyhow::Result<()> {
        let mut set = JoinSet::new();
        for task in self.tasks {
            set.spawn_blocking(|| task.run());
        }

        let mut errors = false;
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

    pub fn retain(&mut self, dir_names: &[String]) {
        self.tasks.retain(|task| {
            let var = &task
                .cwd
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            dir_names.contains(var)
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
        let first = config::Group {
            run_for_dirs: vec![PathBuf::from("some/foo1"), PathBuf::from("some/foo2")],
            commands: vec![
                config::Command::Custom {
                    ignore_if_cwd_ends_with: None,
                    cmd: vec!["cargo".to_string(), "check".to_string()],
                    env: Some(HashMap::from_iter(vec![(
                        "FOO".to_string(),
                        "BAR".to_string(),
                    )])),
                },
                config::Command::MdCheck {
                    ignore_if_in_dir: None,
                },
            ],
        };
        let second = config::Group {
            run_for_dirs: vec![PathBuf::from("some/boo")],
            commands: vec![
                config::Command::Custom {
                    ignore_if_cwd_ends_with: None,
                    cmd: vec!["cargo".to_string(), "test".to_string()],
                    env: None,
                },
                config::Command::Custom {
                    ignore_if_cwd_ends_with: None,
                    cmd: vec!["cargo".to_string(), "fmt".to_string()],
                    env: None,
                },
            ],
        };

        // when
        let mut tasks = Tasks::from_config(config::Config(vec![first, second]), ".");

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
        let config = config::Config(vec![
            config::Group {
                run_for_dirs: vec![PathBuf::from("some/foo"), PathBuf::from("other/zoo")],
                commands: vec![
                    config::Command::Custom {
                        ignore_if_cwd_ends_with: None,
                        cmd: vec!["cargo".to_string(), "check".to_string()],
                        env: None,
                    },
                    config::Command::Custom {
                        ignore_if_cwd_ends_with: None,
                        cmd: vec!["cargo".to_string(), "fmt".to_string()],
                        env: None,
                    },
                    config::Command::Custom {
                        ignore_if_cwd_ends_with: None,
                        cmd: vec!["cargo".to_string(), "test".to_string()],
                        env: None,
                    },
                ],
            },
            config::Group {
                run_for_dirs: vec![PathBuf::from("some/boo")],
                commands: vec![
                    config::Command::Custom {
                        ignore_if_cwd_ends_with: None,
                        cmd: vec!["cargo".to_string(), "check".to_string()],
                        env: None,
                    },
                    config::Command::Custom {
                        ignore_if_cwd_ends_with: None,
                        cmd: vec!["cargo".to_string(), "fmt".to_string()],
                        env: None,
                    },
                    config::Command::Custom {
                        ignore_if_cwd_ends_with: None,
                        cmd: vec!["cargo".to_string(), "test".to_string()],
                        env: None,
                    },
                ],
            },
        ]);

        let mut tasks = Tasks::from_config(config, ".");

        // when
        tasks.retain(&["zoo".to_string()]);

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
        let config = config::Config(vec![config::Group {
            run_for_dirs: vec![PathBuf::from("some/foo")],
            commands: vec![config::Command::Custom {
                ignore_if_cwd_ends_with: None,
                cmd: vec!["cargo".to_string(), "check".to_string()],
                env: None,
            }],
        }]);

        // when
        let mut tasks = Tasks::from_config(config, "workspace");

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
        let config = config::Config(vec![config::Group {
            run_for_dirs: vec![PathBuf::from("boom/some/foo")],
            commands: vec![config::Command::Custom {
                ignore_if_cwd_ends_with: Some(vec!["some/foo".to_string()]),
                cmd: vec!["cargo".to_string(), "check".to_string()],
                env: None,
            }],
        }]);

        // when
        let tasks = Tasks::from_config(config, ".");

        // then
        assert_eq!(tasks.tasks, []);
    }
}
