use std::io::IsTerminal;

use clap::Parser;
mod cli;
mod config;
mod md_check;
mod tasks;
mod util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    util::configure_child_process_cleanup()?;

    if cli.print_ci_jobs_desc {
        let tasks = config::normal(cli.root.clone());
        let jobs = tasks.ci_jobs();
        // Json used because the CI needs it as such
        let jsonified = serde_json::to_string_pretty(&jobs)?;
        println!("{jsonified}");
        return Ok(());
    }

    let tasks = util::generate_tasks(&cli);

    if cli.list_tasks {
        println!("{tasks}");
        return Ok(());
    }

    let is_tty = std::io::stderr().is_terminal();

    let cancel_token = tokio_util::sync::CancellationToken::new();
    util::watch_for_cancel(cancel_token.clone());

    tasks.run(is_tty, cli.verbose, cancel_token).await?;

    Ok(())
}
