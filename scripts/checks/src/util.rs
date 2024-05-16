use nix::unistd::Pid;

use crate::{cli, config, task::Tasks};

pub fn read_tasks_from_config(cli: &cli::Cli) -> Tasks {
    let config_fn = match cli.flavor {
        cli::Flavor::Ci => config::description::ci,
        cli::Flavor::Other => config::description::other,
    };
    let config = config_fn(cli.root.clone(), cli.sway_with_type_paths);

    let mut tasks = Tasks::from_task_descriptions(config, cli.root.clone());

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
