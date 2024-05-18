use nix::unistd::Pid;

use crate::{cli, task::Tasks};

pub fn generate_tasks(cli: &cli::Cli) -> Tasks {
    let tasks_gen = match cli.flavor {
        cli::Flavor::Ci => crate::description::ci,
        cli::Flavor::HackFeatures => crate::description::hack_features,
        cli::Flavor::HackDeps => crate::description::hack_deps,
    };
    let mut tasks = Tasks {
        tasks: tasks_gen(cli.root.clone(), cli.sway_with_type_paths)
            .into_iter()
            .collect(),
    };

    if !cli.only_tasks_with_ids.is_empty() {
        tasks.retain_with_ids(&cli.only_tasks_with_ids);
    }

    if !cli.only_tasks_in_dir.is_empty() {
        tasks.retain_with_dirs(&cli.only_tasks_in_dir);
    }

    tasks
}

pub fn watch_for_cancel(cancel_token: tokio_util::sync::CancellationToken) {
    tokio::task::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        cancel_token.cancel();
    });
}

pub fn configure_child_process_cleanup() -> anyhow::Result<()> {
    // This process is moved into its own process group so that it's easier to kill any of its children.
    nix::unistd::setpgid(Pid::from_raw(0), Pid::from_raw(0))?;
    Ok(())
}
