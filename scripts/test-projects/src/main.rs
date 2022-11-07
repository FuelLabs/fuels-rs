//! Build or format all projects under the main test suite with `forc`/`forc-fmt`
//!
//! NOTE: This expects `forc`, `forc-fmt` and `cargo` to be available in `PATH`.

use clap::Parser;
use futures_util::StreamExt;
use std::path::PathBuf;
use test_projects::{run_recursively, BuildResult, Cli, Command2Run, Commands, ResultWriter};

const TESTS_PATH: &str = "packages/fuels/tests/";

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let mut result_writer = ResultWriter::new();

    let project_path = cli
        .projects_path
        .unwrap_or_else(|| PathBuf::from(TESTS_PATH));
    let project_path = project_path.canonicalize().unwrap_or_else(|_| {
        panic!(
            "project path
            {:?} could not be canonicalized",
            project_path
        )
    });

    let bin_path = if let Some(bin_path) = cli.bin_path {
        bin_path
            .canonicalize()
            .unwrap_or_else(|_| panic!("bin path {:?} could not be canonicalized", bin_path))
    } else {
        PathBuf::from("")
    };

    let command_2_run = match cli.command {
        Commands::Clean => {
            let command = bin_path.join("forc").display().to_string();

            Command2Run {
                command,
                args: vec!["clean".into(), "--path".into()],
                info: "clean".into(),
            }
        }
        Commands::Build => {
            let command = bin_path.join("forc").display().to_string();

            Command2Run {
                command,
                args: vec!["build".into(), "--path".into()],
                info: "build".into(),
            }
        }
        Commands::Format { check } => {
            let command = bin_path.join("forc-fmt").display().to_string();
            if check {
                Command2Run {
                    command,
                    args: vec!["--check".into(), "--path".into()],
                    info: "check".into(),
                }
            } else {
                Command2Run {
                    command,
                    args: vec!["--path".into()],
                    info: "format".into(),
                }
            }
        }
    };

    result_writer
        .display_info(
            cli.num_concurrent,
            &command_2_run.command,
            &command_2_run.info,
        )
        .expect("could not display build info");

    let build_results: Vec<_> = run_recursively(
        &project_path,
        cli.num_concurrent,
        command_2_run.command,
        command_2_run.args,
    )
    .inspect(|result| {
        result_writer
            .display_result(&project_path, result, &command_2_run.info)
            .expect("could not display build result")
    })
    .collect()
    .await;

    let (succeeded, failed): (Vec<_>, Vec<_>) = build_results
        .into_iter()
        .partition(|result| matches! {result, BuildResult::Success(_)});

    if !failed.is_empty() {
        result_writer
            .display_failed(&project_path, &failed)
            .expect("could not display failed projects");
    }

    result_writer
        .display_stats(succeeded.len(), failed.len())
        .expect("could not display stats");

    if !failed.is_empty() {
        std::process::exit(1);
    }
}
