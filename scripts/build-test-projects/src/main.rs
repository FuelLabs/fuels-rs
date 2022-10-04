//! Runs `forc build` for all projects under the
//! `fuels/tests` directory.
//!
//! NOTE: This expects both `forc` and `cargo` to be available in `PATH`.

use std::{
    fs,
    io::{self, Write},
    path::Path,
};

fn main() {
    let output = std::process::Command::new("forc")
        .args(["--version"])
        .output()
        .expect("failed to run `forc --version`");

    let version = String::from_utf8(output.stdout).expect("failed to parse forc --version output");

    println!("Building projects with: {:?}", version.trim());

    let path = Path::new("packages/fuels/tests/");
    let absolute_path = path.canonicalize().unwrap_or_else(|_| {
        panic!(
            "{path:?} could not be canonicalized.\nAre you running the comand from the root of `fuels-rs`?\n"
        )
    });

    let summary = build_recursively(&absolute_path);

    let successes: u64 = summary.iter().sum();
    let failures = summary.len() as u64 - successes;

    let successes_str = if successes == 1 {
        "success"
    } else {
        "successes"
    };
    let failures_str = if failures == 1 { "failure" } else { "failures" };

    println!(
        "{} {}, {} {}",
        successes, successes_str, failures, failures_str
    );

    if failures > 0 {
        std::process::exit(1);
    }
}

fn build_recursively(path: &Path) -> Vec<u64> {
    let mut summary: Vec<u64> = vec![];

    for res in fs::read_dir(path).expect("failed to walk directory") {
        let entry = match res {
            Ok(entry) => entry,
            _ => continue,
        };
        let child_path = entry.path();
        if !child_path.is_dir() {
            continue;
        } else if !dir_contains_forc_manifest(&child_path) {
            summary.extend(build_recursively(&child_path));
        } else {
            let output = std::process::Command::new("forc")
                .args(["build", "--generate-logged-types", "--path"])
                .arg(&child_path)
                .output()
                .expect("failed to run `forc build` for example project");

            // Print output on failure so we can read it in CI.
            let (success, checkmark, status) = if !output.status.success() {
                io::stdout().write_all(&output.stdout).unwrap();
                io::stdout().write_all(&output.stderr).unwrap();
                (0, "[x]", "failed")
            } else {
                (1, "[✓]", "succeeded")
            };
            println!("  {}: {} {}!", checkmark, child_path.display(), status);

            summary.push(success);
        }
    }
    summary
}

// Check if the given directory contains `Forc.toml` at its root.
fn dir_contains_forc_manifest(path: &Path) -> bool {
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if entry.path().file_name().and_then(|s| s.to_str()) == Some("Forc.toml") {
                return true;
            }
        }
    }
    false
}
