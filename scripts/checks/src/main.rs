use std::io::IsTerminal;

use clap::Parser;
mod cli;
mod config;
mod md_check;
mod new_md_check;
mod task;
mod util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    let verbose = cli.verbose;
    let tasks = util::read_tasks_from_config(cli)?;

    let is_tty = std::io::stderr().is_terminal();

    tasks.run(is_tty, verbose).await?;

    Ok(())
}
