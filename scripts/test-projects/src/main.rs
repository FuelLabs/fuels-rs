//! Build or format all projects under the main test suite with `forc`/`forc-fmt`
//!
//! NOTE: This expects `forc`, `forc-fmt` and `cargo` to be available in `PATH`.

use clap::Parser;
use futures_util::StreamExt;
use std::path::Path;
use test_projects::{
    build_recursively, format_recursively, BuildResult, Cli, Commands, ResultWriter,
};

const TESTS_PATH: &str = "packages/fuels/tests/";

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let mut result_writer = ResultWriter::new();

    let path = Path::new(TESTS_PATH);
    let absolute_path = path.canonicalize().unwrap_or_else(|_| {
        panic!(
            "{path:?} could not be canonicalized.\n
            Are you running the comand from the root of `fuels-rs`?\n"
        )
    });

    let (succeeded, failed) = match cli.command {
        Commands::Build { clean } => {
            result_writer
                .display_build_info(cli.num_concurrent, clean)
                .expect("could not display build info");

            let build_results: Vec<_> =
                build_recursively(&absolute_path, cli.num_concurrent, clean)
                    .inspect(|result| {
                        result_writer
                            .display_build_result(&absolute_path, result, clean)
                            .expect("could not display build result")
                    })
                    .collect()
                    .await;

            let (succeeded, failed): (Vec<_>, Vec<_>) = build_results
                .into_iter()
                .partition(|result| matches! {result, BuildResult::Success(_)});

            (succeeded, failed)
        }
        Commands::Format { check } => {
            result_writer
                .display_format_info(cli.num_concurrent, check)
                .expect("could not display format info");

            let build_results: Vec<_> =
                format_recursively(&absolute_path, cli.num_concurrent, check)
                    .inspect(|result| {
                        result_writer
                            .display_format_result(&absolute_path, result, check)
                            .expect("could not display build result")
                    })
                    .collect()
                    .await;

            let (succeeded, failed): (Vec<_>, Vec<_>) = build_results
                .into_iter()
                .partition(|result| matches! {result, BuildResult::Success(_)});

            (succeeded, failed)
        }
    };

    if !failed.is_empty() {
        result_writer
            .display_failed(&absolute_path, &failed)
            .expect("could not display failed projects");
    }

    result_writer
        .display_stats(succeeded.len(), failed.len())
        .expect("could not display stats");

    if !failed.is_empty() {
        std::process::exit(1);
    }
}
