use std::{
    collections::BTreeSet,
    fmt::Display,
    path::{Path, PathBuf},
};

use itertools::Itertools;
use nix::{sys::signal::Signal, unistd::Pid, NixPath};
use sha2::Digest;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

pub mod builder;
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
            writeln!(f, "{task}")?;
        }
        Ok(())
    }
}

impl Tasks {
    pub fn new(
        tasks: impl IntoIterator<Item = task::Task>,
        workspace_root: impl AsRef<Path>,
    ) -> Self {
        Self {
            tasks: BTreeSet::from_iter(tasks),
            workspace_root: workspace_root.as_ref().canonicalize().unwrap(),
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
                () = cancel_token.cancelled() => {
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
