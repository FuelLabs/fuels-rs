use anyhow::{anyhow, bail};
use forc_pkg::{BuildOpts, Built, PkgOpts};
use forc_util::{default_output_directory, find_manifest_dir};
use futures_util::{stream, Stream, StreamExt};
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::types::ReturnValue;
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
    command: String,
    args: Vec<String>,
) -> impl Stream<Item = BuildResult> {
    stream::iter(discover_projects(path))
        .map(move |path| {
            let command: String = command.clone();
            let args: Vec<String> = args.clone();
            let path_string: String = path.display().to_string();
            async move {
                let output = match (command.as_str(), args.get(0).unwrap().as_str()) {
                    ("forc", "build") => ReturnValue::Some(build(Some(path_string))),
                    ("forc", "clean") => ReturnValue::Clean(clean(Some(path_string))),
                    ("forc-fmt", _) => ReturnValue::Format(fmt(path.clone(), command, args).await),
                    _ => ReturnValue::Clean(Err(anyhow!("no command"))),
                };

                match output {
                    ReturnValue::Some(output) => {
                        if output.is_ok() {
                            let compilation_result = BuildOutput {
                                path,
                                stderr: String::new(),
                            };
                            BuildResult::Success(compilation_result)
                        } else {
                            let compilation_result = BuildOutput {
                                path,
                                stderr: output.err().map(|err| err.to_string()).unwrap_or_default(),
                            };
                            BuildResult::Failure(compilation_result)
                        }
                    }
                    ReturnValue::Clean(output) => {
                        if output.is_ok() {
                            let compilation_result = BuildOutput {
                                path,
                                stderr: String::new(),
                            };
                            BuildResult::Success(compilation_result)
                        } else {
                            let compilation_result = BuildOutput {
                                path,
                                stderr: output.err().map(|err| err.to_string()).unwrap_or_default(),
                            };
                            BuildResult::Failure(compilation_result)
                        }
                    }
                    ReturnValue::Format(_) => {
                        let compilation_result = BuildOutput {
                            path,
                            stderr: String::new(),
                        };
                        BuildResult::Success(compilation_result)
                    }
                }
            }
        })
        .buffer_unordered(1)
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

pub fn build(path: Option<String>) -> anyhow::Result<Built> {
    let pkg_opts = forc_pkg::PkgOpts {
        path,
        ..PkgOpts::default()
    };

    let build_opts = BuildOpts {
        pkg: pkg_opts,
        ..BuildOpts::default()
    };

    forc_pkg::build_with_options(build_opts)
}

pub fn clean(path: Option<String>) -> anyhow::Result<()> {
    // let path = Some("packages/fuels/tests/types/b512test".to_string());

    // find manifest directory, even if in subdirectory
    let this_dir = if let Some(ref path) = path {
        PathBuf::from(path)
    } else {
        std::env::current_dir().map_err(|e| anyhow!("{:?}", e))?
    };
    let manifest_dir = match find_manifest_dir(&this_dir) {
        Some(dir) => dir,
        None => {
            bail!(
                "could not find `{}` in `{}` or any parent directory",
                "Forc.toml",
                this_dir.display(),
            )
        }
    };

    // Clear `<project>/out` directory.
    // Ignore I/O errors telling us `out_dir` isn't there.
    let out_dir = default_output_directory(&manifest_dir);
    let _ = std::fs::remove_dir_all(out_dir);

    Ok(())
}

pub async fn fmt(path: PathBuf, command: String, args: Vec<String>) -> BuildResult {
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
