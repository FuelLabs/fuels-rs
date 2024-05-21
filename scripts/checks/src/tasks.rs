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

use self::ci_job::CiJob;

pub mod builder;
pub mod ci_job;
pub mod command;
pub mod deps;
pub mod report;
pub mod task;

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

    pub fn ci_jobs(&self) -> Vec<CiJob> {
        // tasks grouped by dir to reuse compilation artifacts and shorten CI time
        self.tasks
            .iter()
            .sorted_by_key(|task| task.cwd.clone())
            .group_by(|task| task.cwd.clone())
            .into_iter()
            .flat_map(|(cwd, tasks)| {
                let (tasks_requiring_type_paths, normal_tasks) =
                    separate_out_type_path_tasks(tasks);

                let name = self.create_job_name(cwd);

                // You cannot have type paths and not have them in the same job, so they need to be
                // separate jobs.
                [
                    job_with_merged_deps(&tasks_requiring_type_paths, name.clone()),
                    job_with_merged_deps(&normal_tasks, name),
                ]
                .into_iter()
                .flatten()
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
                Some(deps::Sway::Normal) | None
            )
        });
    }

    fn create_job_name(&self, cwd: PathBuf) -> String {
        // So we don't take up much real estate printing the full canonicalized path
        let relative_path = cwd.strip_prefix(&self.workspace_root).unwrap_or_else(|_| {
            panic!(
                "expected {cwd:?} to be a prefix of {}",
                self.workspace_root.display()
            )
        });

        if relative_path.is_empty() {
            "workspace".to_string()
        } else {
            format!("{}", relative_path.display())
        }
    }
}

fn job_with_merged_deps(tasks: &[&task::Task], name: String) -> Option<CiJob> {
    tasks
        .iter()
        .map(|ty| ty.cmd.deps())
        .reduce(|acc, next| acc + next)
        .map(|dep| CiJob::new(dep, tasks, name))
}

fn separate_out_type_path_tasks<'a>(
    tasks: impl IntoIterator<Item = &'a task::Task>,
) -> (Vec<&'a task::Task>, Vec<&'a task::Task>) {
    tasks.into_iter().partition(|task| {
        task.cmd
            .deps()
            .sway_artifacts
            .is_some_and(|dep| matches!(dep, deps::Sway::TypePaths))
    })
}
