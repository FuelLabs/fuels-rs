//! Build or format all projects under the main test suite with `forc`/`forc-fmt`
//!
//! NOTE: This expects `forc`, `forc-fmt` and `cargo` to be available in `PATH`.

use test_projects::cli::parse_cli;
use test_projects::types::ResultWriter;
use test_projects::{check_workspace, display_info, run_command};

#[tokio::main]
async fn main() {
    let mut result_writer = ResultWriter::new();
    let run_config = parse_cli();
    check_workspace(&run_config.project_path, &mut result_writer)
        .expect("Failed to check workspace");
    let output_result = run_command(&run_config).await;
    display_info(&mut result_writer, &run_config, output_result);
}
