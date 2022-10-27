//! Runs `forc build` for all projects under the
//! `fuels/tests` directory.
//!
//! NOTE: This expects both `forc` and `cargo` to be available in `PATH`.

mod lib;

use crate::lib::{build_recursively, BuildResult, ResultWriter};
use futures_util::StreamExt;
use std::{env, path::Path};

const TESTS_PATH: &str = "packages/fuels/tests/";
const DEF_NUM_CONC_BUILDS: usize = 1;

#[tokio::main]
async fn main() {
    let mut result_writer = ResultWriter::new();

    let num_buf_futures: usize = env::var("NUM_CONC_BUILDS")
        .ok()
        .and_then(|e| e.parse().ok())
        .unwrap_or(DEF_NUM_CONC_BUILDS);

    result_writer
        .display_build_info(num_buf_futures)
        .expect("could not display build info");

    let path = Path::new(TESTS_PATH);
    let absolute_path = path.canonicalize().unwrap_or_else(|_| {
        panic!(
            "{path:?} could not be canonicalized.\n
            Are you running the comand from the root of `fuels-rs`?\n"
        )
    });

    let build_results: Vec<_> = build_recursively(&absolute_path, num_buf_futures)
        .inspect(|result| {
            result_writer
                .display_result(&absolute_path, result)
                .expect("could not display build result")
        })
        .collect()
        .await;

    let (succeeded, failed): (Vec<_>, Vec<_>) = build_results
        .into_iter()
        .partition(|result| matches! {result, BuildResult::Success(_)});

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
