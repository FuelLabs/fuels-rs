use std::io::IsTerminal;

use clap::Parser;
mod cli;
mod config;
mod md_check;
mod task;
mod util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();

    let tasks = util::read_tasks_from_config(&cli);

    if cli.list_tasks {
        for task in &tasks.tasks {
            println!("{task}");
        }
        return Ok(());
    }

    if cli.list_used_dirs {
        let dirs = tasks.used_dirs();
        // Json used because the CI needs it as such
        println!("{}", serde_json::json!({ "dirs": dirs }));
        return Ok(());
    }

    let is_tty = std::io::stderr().is_terminal();

    let cancel_token = tokio_util::sync::CancellationToken::new();
    util::watch_for_cancel(cancel_token.clone());

    tasks.run(is_tty, cli.verbose, cancel_token).await?;

    Ok(())
}
