use std::{
    fs,
    path::{Path, PathBuf},
};

use futures_util::{stream, Stream, StreamExt};

use crate::{
    cli::RunConfig,
    types::{BuildOutput, BuildResult, ResultWriter},
};

pub mod cli;
pub mod types;

pub async fn run_command(
    result_writer: &mut ResultWriter,
    config: &RunConfig,
) -> (Vec<BuildResult>, Vec<BuildResult>) {
    let build_results: Vec<_> = run_recursively(
        &config.project_path,
        config.num_concurrent,
        config.prepared_command.command.clone(),
        config.prepared_command.args.clone(),
    )
    .inspect(|result| {
        result_writer
            .display_result(&config.project_path, result, &config.prepared_command.info)
            .expect("could not display build result")
    })
    .collect()
    .await;

    build_results
        .into_iter()
        .partition(|result| matches! {result, BuildResult::Success(_)})
}

pub fn display_info(result_writer: &mut ResultWriter, config: &RunConfig) {
    result_writer
        .display_info(config)
        .expect("could not display build info");
}

pub fn display_failed(
    result_writer: &mut ResultWriter,
    config: &RunConfig,
    failed: &[BuildResult],
) {
    if !failed.is_empty() {
        result_writer
            .display_failed(&config.project_path, failed)
            .expect("could not display failed projects");
    }
}

pub fn display_stats(
    result_writer: &mut ResultWriter,
    succeeded: &[BuildResult],
    failed: &[BuildResult],
) {
    result_writer
        .display_stats(succeeded.len(), failed.len())
        .expect("could not display stats");
}

pub fn discover_projects(path: &Path) -> Vec<PathBuf> {
    fs::read_dir(path)
        .expect("failed to walk directory")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .flat_map(|path| {
            if dir_contains_forc_manifest(&path) {
                vec![path]
            } else {
                discover_projects(&path)
            }
        })
        .collect()
}

pub fn run_recursively(
    path: &Path,
    num_conc_futures: usize,
    command: String,
    args: Vec<String>,
) -> impl Stream<Item = BuildResult> {
    stream::iter(discover_projects(path))
        .map(move |path| {
            let command = command.clone();
            let args = args.clone();

            async move {
                let output = tokio::process::Command::new(command)
                    .args(args)
                    .arg(&path)
                    .output()
                    .await
                    .expect("failed to run command");

                let compilation_result = BuildOutput {
                    path,
                    stderr: String::from_utf8(output.stderr).expect("stderr is not valid utf8"),
                };

                if output.status.success() {
                    BuildResult::Success(compilation_result)
                } else {
                    BuildResult::Failure(compilation_result)
                }
            }
        })
        .buffer_unordered(num_conc_futures)
}

// Check if the given directory contains `Forc.toml` at its root.
pub fn dir_contains_forc_manifest(path: &Path) -> bool {
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if entry.path().file_name().and_then(|s| s.to_str()) == Some("Forc.toml") {
                return true;
            }
        }
    }
    false
}
