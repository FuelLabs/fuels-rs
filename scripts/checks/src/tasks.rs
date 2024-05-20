use std::{collections::BTreeSet, fmt::Display, path::PathBuf};

use itertools::Itertools;
use nix::{sys::signal::Signal, unistd::Pid, NixPath};
use sha2::Digest;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

pub mod command;
pub mod deps;
pub mod report;
pub mod task;

use self::deps::SwayArtifacts;

fn short_sha256(input: &str) -> String {
    let mut hasher = sha2::Sha256::default();
    hasher.update(input.as_bytes());
    hex::encode(&hasher.finalize()[..8])
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tasks {
    tasks: BTreeSet<task::Task>,
    workspace_root: PathBuf,
}

impl Display for Tasks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for task in &self.tasks {
            writeln!(f, "{}", task)?;
        }
        Ok(())
    }
}

impl Tasks {
    pub fn new(tasks: BTreeSet<task::Task>, workspace_root: PathBuf) -> Self {
        Self {
            tasks,
            workspace_root: workspace_root.canonicalize().unwrap(),
        }
    }

    pub fn ci_jobs(&self) -> Vec<deps::CiJob> {
        self.tasks
            .iter()
            .sorted_by_key(|task| task.cwd.clone())
            .group_by(|task| task.cwd.clone())
            .into_iter()
            .flat_map(|(cwd, tasks)| {
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

                let relative_path = cwd.strip_prefix(&self.workspace_root).unwrap_or_else(|_| {
                    panic!("{cwd:?} is a prefix of {}", self.workspace_root.display())
                });

                let name = if relative_path.is_empty() {
                    "workspace".to_string()
                } else {
                    format!("{}", relative_path.display())
                };

                if let Some(deps) = type_paths_deps {
                    jobs.push(deps::CiJob::new(
                        deps,
                        &tasks_requiring_type_paths,
                        name.clone(),
                    ));
                }

                if let Some(deps) = normal_deps {
                    jobs.push(deps::CiJob::new(deps, &normal_tasks, name));
                }

                jobs
            })
            .collect()
    }

    pub async fn run(
        self,
        tty: bool,
        verbose: bool,
        cancel_token: CancellationToken,
    ) -> anyhow::Result<()> {
        let mut set = JoinSet::new();
        for task in self.tasks {
            set.spawn_blocking(|| task.run());
        }

        let mut errors = false;

        let mut handle_task_response = |execution: report::Report| {
            if let report::Status::Failed { .. } = execution.status {
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

    pub fn retain_with_dirs(&mut self, dirs: &[PathBuf]) {
        let dirs = dirs
            .iter()
            .map(|dir| {
                dir.canonicalize()
                    .unwrap_or_else(|_| panic!("unable to canonicalize path {dir:?}"))
            })
            .collect_vec();
        self.tasks.retain(|task| dirs.contains(&task.cwd));
    }

    pub fn retain_without_type_paths(&mut self) {
        self.tasks.retain(|task| {
            matches!(
                task.cmd.deps().sway_artifacts,
                Some(SwayArtifacts::Normal) | None
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
