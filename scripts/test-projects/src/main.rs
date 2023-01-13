//! Build or format all projects under the main test suite with `forc`/`forc-fmt`
//!
//! NOTE: This expects `forc`, `forc-fmt` and `cargo` to be available in `PATH`.

use anyhow::Error;
use test_projects::{
    cli::parse_cli, display_failed, display_stats, run_command, types::ResultWriter,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut result_writer = ResultWriter::new();
    let run_config = parse_cli();

    let (succeded, failed) = run_command(&mut result_writer, &run_config).await;

    display_failed(&mut result_writer, &run_config, &failed);
    display_stats(&mut result_writer, &succeded, &failed);

    if !failed.is_empty() {
        std::process::exit(1);
    }

    Ok(())
}
