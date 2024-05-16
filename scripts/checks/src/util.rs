use crate::{cli, config, task::Tasks};

pub fn read_tasks_from_config(cli: &cli::Cli) -> Tasks {
    let config = match cli.flavor {
        cli::Flavor::Ci => config::ci::ci_config(cli.sway_with_type_paths),
        cli::Flavor::Max => todo!(),
    };

    let mut tasks = Tasks::from_task_descriptions(config, cli.workspace_root.clone());

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
