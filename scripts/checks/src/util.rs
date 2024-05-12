use crate::{cli, config, task::Tasks};

pub fn read_tasks_from_config(cli: cli::Cli) -> anyhow::Result<Tasks> {
    let yml = match cli.flavor {
        cli::Flavor::Ci => include_str!("../config/ci.yml"),
        cli::Flavor::Max => include_str!("../config/max.yml"),
    };
    let config = serde_yaml::from_str::<config::Config>(yml)?;

    let mut tasks = Tasks::from_config(config, cli.workspace_root);

    if !cli.crates.is_empty() {
        tasks.retain(&cli.crates);
    }

    Ok(tasks)
}
