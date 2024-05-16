use std::path::PathBuf;

use crate::config::RunIf;
use crate::config::TasksDescription;

use super::Command;

pub fn ci_config(sway_type_paths: bool) -> Vec<TasksDescription> {
    vec![
        common(),
        e2e_specific(sway_type_paths),
        wasm_specific(),
        workspace_level(),
    ]
}

fn paths(paths: &[&str]) -> Vec<PathBuf> {
    let workspace = PathBuf::from(file!())
        .parent()
        .unwrap()
        .join("../../../../");

    paths
        .iter()
        .map(|path| workspace.join(path).canonicalize().unwrap())
        .collect()
}

fn split(string: &str) -> Vec<String> {
    string
        .split_whitespace()
        .map(|word| word.to_owned())
        .collect()
}

macro_rules! custom {
    ($cmd: literal) => {
        crate::config::Command::Custom {
            cmd: self::split($cmd),
            env: None,
            run_if: None,
        }
    };
    ($cmd: literal, $run_if: expr) => {
        crate::config::Command::Custom {
            cmd: self::split($cmd),
            env: None,
            run_if: Some($run_if),
        }
    };
    ($cmd: literal, $env: literal) => {
        crate::config::Command::Custom {
            cmd: self::split($cmd),
            env: Some($env),
            run_if: None,
        }
    };
    ($cmd: literal , $run_if: expr,  $($env_key:literal = $env_value:literal),*) => {
        crate::config::Command::Custom {
            cmd: self::split($cmd),
            env: Some(
            std::collections::HashMap::from_iter([
                $(($env_key.to_owned(), $env_value.to_owned()),)*
            ])),
            run_if: Some($run_if),
        }
    };
}

fn cwd_doesnt_end_with(suffixes: &[&str]) -> RunIf {
    RunIf::CwdDoesntEndWith(suffixes.iter().map(|s| s.to_string()).collect())
}

fn common() -> TasksDescription {
    TasksDescription {
        run_for_dirs: paths(&[
            "packages/fuels",
            "packages/fuels-accounts",
            "packages/fuels-code-gen",
            "packages/fuels-core",
            "packages/fuels-macros",
            "packages/fuels-programs",
            "packages/fuels-test-helpers",
            "e2e",
            "wasm-tests",
            "scripts/checks",
        ]),
        commands: vec![
            custom!("cargo fmt --verbose --check"),
            custom!("typos"),
            custom!("cargo clippy --all-targets --all-features --no-deps"),
            custom!(
                "cargo nextest run --all-features",
                // e2e ignored because we have to control the features carefully (e.g. rocksdb, test-type-paths, etc)
                // wasm ignored because wasm tests need to be run with wasm-pack
                cwd_doesnt_end_with(&["wasm-tests", "e2e"])
            ),
            custom!(
                "cargo test --doc",
                // because these don't have libs
                cwd_doesnt_end_with(&["e2e", "scripts/checks", "wasm-tests"]),
                "RUSTDOCFLAGS" = "-D warnings"
            ),
            custom!(
                "cargo doc --document-private-items",
                // because these don't have libs
                cwd_doesnt_end_with(&["e2e", "scripts/checks", "wasm-tests"]),
                "RUSTDOCFLAGS" = "-D warnings"
            ),
        ],
    }
}

fn e2e_specific(sway_type_paths: bool) -> TasksDescription {
    let commands = if sway_type_paths {
        vec![custom!(
            "cargo nextest run --features default,fuel-core-lib,test-type-paths"
        )]
    } else {
        vec![
            custom!("cargo nextest run --features default,fuel-core-lib"),
            custom!("cargo nextest run --features default"),
        ]
    };
    TasksDescription {
        run_for_dirs: paths(&["e2e"]),
        commands,
    }
}

fn wasm_specific() -> TasksDescription {
    TasksDescription {
        run_for_dirs: paths(&["wasm-tests"]),
        commands: vec![custom!("wasm-pack test --node")],
    }
}

fn workspace_level() -> TasksDescription {
    TasksDescription {
        run_for_dirs: paths(&["."]),
        commands: vec![
            Command::MdCheck { run_if: None },
            custom!("cargo machete --skip-target-dir"),
            custom!("cargo clippy --workspace --all-features"),
            custom!("typos"),
        ],
    }
}
