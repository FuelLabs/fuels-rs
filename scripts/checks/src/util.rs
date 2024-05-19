use nix::unistd::Pid;

use crate::{cli, description, task::Tasks};

pub fn generate_tasks(cli: &cli::Cli) -> Tasks {
    let mut tasks = match cli.flavor {
        cli::Flavor::Normal => description::normal(cli.root.clone()),
        cli::Flavor::HackFeatures => description::hack_features(cli.root.clone()),
        cli::Flavor::HackDeps => description::hack_deps(cli.root.clone()),
    };

    if let Some(ids) = &cli.only_tasks_with_ids {
        tasks.retain_with_ids(ids);
    }

    if let Some(dirs) = &cli.only_tasks_in_dir {
        tasks.retain_with_dirs(dirs);
    }

    if !cli.sway_type_paths {
        tasks.retain_without_type_paths();
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
