use itertools::Itertools;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use crate::config::RunIf;
use crate::config::TasksDescription;

use super::Command;

pub fn ci(workspace: PathBuf, sway_type_paths: bool) -> Vec<TasksDescription> {
    let desc = TaskDescriptionBuilder::new(workspace, sway_type_paths, &["-Dwarnings"]);
    vec![
        desc.common(),
        desc.e2e_specific(),
        desc.wasm_specific(),
        desc.workspace_level(),
    ]
}

pub fn other(workspace: PathBuf, sway_type_paths: bool) -> Vec<TasksDescription> {
    let desc = TaskDescriptionBuilder::new(
        workspace,
        sway_type_paths,
        &["-Dwarnings", "-Dunused_crate_dependencies"],
    );
    vec![desc.hack_common(), desc.hack_e2e()]
}

struct TaskDescriptionBuilder {
    workspace: PathBuf,
    sway_type_paths: bool,
    rust_flags: Vec<String>,
}

impl TaskDescriptionBuilder {
    fn new(workspace: PathBuf, sway_type_paths: bool, rust_flags: &[&str]) -> Self {
        Self {
            workspace,
            sway_type_paths,
            rust_flags: rust_flags.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn workspace_path(&self, paths: &[&str]) -> Vec<PathBuf> {
        paths
            .iter()
            .map(|path| {
                let path = self.workspace.join(path);
                path.canonicalize()
                    .unwrap_or_else(|_| panic!("Path not found: {:?}", path))
            })
            .collect()
    }

    fn hack_common(&self) -> TasksDescription {
        TasksDescription {
            run_for_dirs: all_workspace_members(&self.workspace),
            commands: vec![
                self.cargo_if(
                    "hack --feature-powerset check",
                    cwd_doesnt_end_with(&["e2e"]),
                ),
                self.cargo_if(
                    "hack --feature-powerset check --tests",
                    cwd_doesnt_end_with(&["e2e"]),
                ),
            ],
        }
    }

    fn hack_e2e(&self) -> TasksDescription {
        let exclude_features = if self.sway_type_paths {
            ""
        } else {
            "--exclude-features test-type-paths"
        };
        TasksDescription {
            run_for_dirs: self.workspace_path(&["e2e"]),
            commands: vec![
                self.cargo(format!("hack --feature-powerset {exclude_features} check ")),
                self.cargo(format!(
                    "hack --feature-powerset {exclude_features} check --tests"
                )),
            ],
        }
    }

    fn cargo(&self, cmd: impl Into<String>) -> Command {
        self.cargo_full(cmd, None, None)
    }

    fn cargo_if(&self, cmd: impl Into<String>, run_if: Option<RunIf>) -> Command {
        self.cargo_full(cmd, None, run_if)
    }

    fn cargo_full(
        &self,
        cmd: impl Into<String>,
        env: Option<(&str, &str)>,
        run_if: Option<RunIf>,
    ) -> Command {
        let mut envs = self.rust_flags_env();

        if let Some(env) = env {
            envs.insert(env.0.into(), env.1.into());
        }

        Command::Custom {
            cmd: parse_cmd("cargo", &cmd.into()),
            env: Some(envs),
            run_if,
        }
    }

    fn rust_flags_env(&self) -> HashMap<String, String> {
        let value = self.rust_flags.iter().join(",");
        HashMap::from_iter(vec![("RUSTFLAGS".to_owned(), value)])
    }

    fn custom(&self, cmd: &str) -> Command {
        Command::Custom {
            cmd: parse_cmd("", cmd),
            env: None,
            run_if: None,
        }
    }

    fn e2e_specific(&self) -> TasksDescription {
        let commands = if self.sway_type_paths {
            vec![
                self.cargo("nextest run --features default,fuel-core-lib,test-type-paths"),
                self.cargo("clippy --all-features --all-targets --no-deps"),
            ]
        } else {
            vec![
                self.cargo("nextest run --features default"),
                self.cargo("clippy --features default,fuel-core-lib --all-targets --no-deps"),
            ]
        };
        TasksDescription {
            run_for_dirs: self.workspace_path(&["e2e"]),
            commands,
        }
    }

    fn wasm_specific(&self) -> TasksDescription {
        TasksDescription {
            run_for_dirs: self.workspace_path(&["wasm-tests"]),
            commands: vec![self.custom("wasm-pack test --node")],
        }
    }

    fn workspace_level(&self) -> TasksDescription {
        TasksDescription {
            run_for_dirs: self.workspace_path(&["."]),
            commands: vec![
                Command::MdCheck { run_if: None },
                self.custom("cargo-machete --skip-target-dir"),
                self.cargo("clippy --workspace --all-features"),
                self.custom("typos"),
            ],
        }
    }

    fn common(&self) -> TasksDescription {
        let workspace = &self.workspace;
        TasksDescription {
            run_for_dirs: all_workspace_members(workspace),
            commands: vec![
                self.cargo("fmt --verbose --check"),
                self.custom("typos"),
                self.cargo_if(
                    "clippy --all-targets --all-features --no-deps",
                    // e2e ignored because we have to control the features carefully (e.g. rocksdb, test-type-paths, etc)
                    cwd_doesnt_end_with(&["e2e"]),
                ),
                self.cargo_if(
                    "nextest run --all-features",
                    // e2e ignored because we have to control the features carefully (e.g. rocksdb, test-type-paths, etc)
                    // wasm ignored because wasm tests need to be run with wasm-pack
                    cwd_doesnt_end_with(&["wasm-tests", "e2e"]),
                ),
                self.cargo_if(
                    "test --doc",
                    // because these don't have libs
                    cwd_doesnt_end_with(&["e2e", "scripts/checks", "wasm-tests"]),
                ),
                self.cargo_full(
                    "doc --document-private-items",
                    Some(("RUSTDOCFLAGS", "-Dwarnings")),
                    // because these don't have libs
                    cwd_doesnt_end_with(&["e2e", "scripts/checks", "wasm-tests"]),
                ),
            ],
        }
    }
}

fn cwd_doesnt_end_with(suffixes: &[&str]) -> Option<RunIf> {
    Some(RunIf::CwdDoesntEndWith(
        suffixes.iter().map(|s| s.to_string()).collect(),
    ))
}

fn parse_cmd(prepend: &str, string: &str) -> Vec<String> {
    let parts = string.split_whitespace().map(|s| s.to_string()).collect();
    if prepend.is_empty() {
        parts
    } else {
        [vec![prepend.to_owned()], parts].concat()
    }
}

include!(concat!(env!("OUT_DIR"), "/workspace_members.rs"));
fn all_workspace_members(workspace: &Path) -> Vec<PathBuf> {
    self::WORKSPACE_MEMBERS
        .iter()
        .map(|member| workspace.join(member).canonicalize().unwrap())
        .collect()
}
