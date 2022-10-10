//! Runs `forc build` for all projects under the
//! `fuels/tests` directory.
//!
//! NOTE: This expects both `forc` and `cargo` to be available in `PATH`.

mod lib;

use crate::lib::{build_recursively, BuildResult, ResultWriter};
use std::path::Path;

const TESTS_PATH: &str = "packages/fuels/tests/";

fn main() {
    let mut result_writer = ResultWriter::new();

    result_writer
        .display_forc_info()
        .expect("could not display forc info");

    let path = Path::new(TESTS_PATH);
    let absolute_path = path.canonicalize().unwrap_or_else(|_| {
        panic!(
            "{path:?} could not be canonicalized.\n
            Are you running the comand from the root of `fuels-rs`?\n"
        )
    });

    let (succeeded, failed): (Vec<_>, Vec<_>) = build_recursively(&absolute_path)
        .into_iter()
        .inspect(|result| {
            result_writer
                .display_result(&absolute_path, result)
                .expect("could not display build result")
        })
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
