use itertools::Itertools;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use crate::tasks::{
    command::Command,
    deps,
    deps::{All, SwayArtifacts},
    task::Task,
    Tasks,
};
pub struct Builder {
    workspace: PathBuf,
    rust_flags: Vec<String>,
    tasks: Vec<Task>,
}

impl Builder {
    pub fn new(workspace: PathBuf, rust_flags: &[&str]) -> Self {
        Self {
            workspace,
            rust_flags: rust_flags.iter().map(|s| (*s).to_string()).collect(),
            tasks: vec![],
        }
    }

    pub fn common(&mut self) {
        let tasks = self
            .all_workspace_members(None)
            .into_iter()
            .flat_map(|member| {
                let deps = {
                    // Some examples run abigen! on sway projects in e2e
                    let sway_artifacts = member
                        .starts_with(self.workspace_path("examples"))
                        .then_some(SwayArtifacts::Normal);

                    All {
                        sway_artifacts,
                        ..Default::default()
                    }
                };

                let mut commands = vec![
                    self.cargo_fmt("--verbose --check", deps.clone()),
                    Self::custom(
                        "typos",
                        "",
                        &All {
                            typos_cli: true,
                            ..deps.clone()
                        },
                    ),
                ];

                // e2e ignored because we have to control the features carefully (e.g. rocksdb, test-type-paths, etc)
                if member != self.workspace_path("e2e") {
                    let cmd =
                        self.cargo_clippy("--all-targets --all-features --no-deps", deps.clone());
                    commands.push(cmd);
                }

                // e2e ignored because we have to control the features carefully (e.g. rocksdb, test-type-paths, etc)
                // wasm ignored because wasm tests need to be run with wasm-pack
                if member != self.workspace_path("wasm-tests")
                    && member != self.workspace_path("e2e")
                {
                    let cmd = self.cargo_nextest("run --all-features", deps.clone());
                    commands.push(cmd);
                }

                // because these don't have libs
                if member != self.workspace_path("e2e")
                    && member != self.workspace_path("wasm-tests")
                    && member != self.workspace_path("scripts/checks")
                {
                    let cmd = self.cargo("test --doc", None, deps.clone());
                    commands.push(cmd);

                    let cmd = self.cargo(
                        "doc --document-private-items",
                        Some(("RUSTDOCFLAGS", "-Dwarnings")),
                        deps,
                    );
                    commands.push(cmd);
                }

                commands.into_iter().map(move |cmd| Task {
                    cwd: member.clone(),
                    cmd,
                })
            })
            .collect_vec();

        self.tasks.extend(tasks);
    }

    pub fn e2e_specific(&mut self) {
        let tasks = [
            self.cargo_nextest(
                "run --features default,fuel-core-lib,test-type-paths",
                All {
                    sway_artifacts: Some(deps::SwayArtifacts::TypePaths),
                    ..Default::default()
                },
            ),
            self.cargo_nextest(
                "run --features default,fuel-core-lib",
                All {
                    sway_artifacts: Some(deps::SwayArtifacts::Normal),
                    ..Default::default()
                },
            ),
            self.cargo_nextest(
                "run --features default,test-type-paths",
                All {
                    fuel_core_binary: true,
                    sway_artifacts: Some(deps::SwayArtifacts::TypePaths),
                    ..Default::default()
                },
            ),
            self.cargo_clippy(
                "--all-targets --no-deps --features default,test-type-paths",
                All {
                    sway_artifacts: Some(deps::SwayArtifacts::TypePaths),
                    ..Default::default()
                },
            ),
        ]
        .map(|cmd| Task {
            cwd: self.workspace_path("e2e"),
            cmd,
        });

        self.tasks.extend(tasks);
    }

    pub fn wasm_specific(&mut self) {
        let task = Task {
            cwd: self.workspace_path("wasm-tests"),
            cmd: Self::custom(
                "wasm-pack",
                "test --node",
                &All {
                    wasm: true,
                    ..Default::default()
                },
            ),
        };
        self.tasks.push(task);
    }

    pub fn workspace_level(&mut self) {
        let tasks = [
            Command::MdCheck,
            Self::custom(
                "cargo-machete",
                "--skip-target-dir",
                &All {
                    cargo: deps::Cargo {
                        machete: true,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ),
            self.cargo_clippy(
                "--workspace --all-features",
                All {
                    sway_artifacts: Some(deps::SwayArtifacts::Normal),
                    ..Default::default()
                },
            ),
            Self::custom(
                "typos",
                "",
                &All {
                    typos_cli: true,
                    ..Default::default()
                },
            ),
        ]
        .map(|cmd| Task {
            cwd: self.workspace_path("."),
            cmd,
        });

        self.tasks.extend(tasks);
    }

    pub fn hack_features_common(&mut self) {
        let ignore = self.workspace_path("e2e");
        let tasks = self
            .all_workspace_members(Some(&ignore))
            .into_iter()
            .flat_map(|member| {
                [
                    self.cargo_hack("--feature-powerset check", All::default()),
                    self.cargo_hack("--feature-powerset check --tests", All::default()),
                ]
                .into_iter()
                .map(move |cmd| Task {
                    cwd: member.clone(),
                    cmd,
                })
            })
            .collect_vec();

        self.tasks.extend(tasks);
    }

    pub fn hack_features_e2e(&mut self) {
        let tasks = [
            self.cargo_hack(
                "--feature-powerset check --tests",
                All {
                    sway_artifacts: Some(deps::SwayArtifacts::TypePaths),
                    ..Default::default()
                },
            ),
            self.cargo_hack(
                "--feature-powerset --exclude-features test-type-paths check --tests",
                All {
                    sway_artifacts: Some(deps::SwayArtifacts::Normal),
                    ..Default::default()
                },
            ),
        ]
        .map(|cmd| Task {
            cwd: self.workspace_path("e2e"),
            cmd,
        })
        .to_vec();

        self.tasks.extend(tasks);
    }

    pub fn hack_deps_common(&mut self) {
        let ignore = self.workspace_path("e2e");
        let tasks = self
            .all_workspace_members(Some(&ignore))
            .into_iter()
            .flat_map(|member| {
                let deps = All {
                    cargo: deps::Cargo {
                        udeps: true,
                        ..Default::default()
                    },
                    rust: Some(deps::Rust {
                        nightly: true,
                        ..Default::default()
                    }),
                    ..Default::default()
                };
                [
                    self.cargo_hack("udeps", deps.clone()),
                    self.cargo_hack("udeps --tests", deps),
                ]
                .into_iter()
                .map(move |cmd| Task {
                    cwd: member.clone(),
                    cmd,
                })
            })
            .collect_vec();

        self.tasks.extend(tasks);
    }

    fn cargo_fmt(&self, cmd: impl Into<String>, mut deps: All) -> Command {
        deps += All {
            rust: Some(deps::Rust {
                components: BTreeSet::from_iter(["rustfmt".to_string()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let cmd = format!("fmt {}", cmd.into());

        self.cargo(cmd, None, deps)
    }

    fn cargo_clippy(&self, cmd: impl Into<String>, mut deps: All) -> Command {
        deps += All {
            rust: Some(deps::Rust {
                components: BTreeSet::from_iter(["clippy".to_string()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let cmd = format!("clippy {}", cmd.into());
        self.cargo(cmd, None, deps)
    }

    fn cargo_hack(&self, cmd: impl Into<String>, mut deps: All) -> Command {
        deps += All {
            cargo: deps::Cargo {
                hack: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let cmd = format!("hack {}", cmd.into());
        self.cargo(cmd, None, deps)
    }

    fn cargo_nextest(&self, cmd: impl Into<String>, mut deps: All) -> Command {
        deps += All {
            cargo: deps::Cargo {
                nextest: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let cmd = format!("nextest {}", cmd.into());

        self.cargo(cmd, None, deps)
    }

    fn cargo(&self, cmd: impl Into<String>, env: Option<(&str, &str)>, deps: All) -> Command {
        let envs = {
            let flags = self.rust_flags.iter().join(" ");
            let mut envs = vec![("RUSTFLAGS".to_owned(), flags)];

            if let Some(env) = env {
                envs.push((env.0.into(), env.1.into()));
            }
            envs
        };

        let nightly = if deps.rust.as_ref().is_some_and(|r| r.nightly) {
            "+nightly"
        } else {
            ""
        };

        Command::Custom {
            program: "cargo".to_string(),
            args: parse_cmd(nightly, &cmd.into()),
            env: envs,
            deps,
        }
    }

    fn custom(program: &str, args: &str, deps: &All) -> Command {
        Command::Custom {
            program: program.to_owned(),
            args: parse_cmd("", args),
            env: vec![],
            deps: deps.clone(),
        }
    }

    fn workspace_path(&self, path: &str) -> PathBuf {
        let path = self.workspace.join(path);
        path.canonicalize()
            .unwrap_or_else(|_| panic!("Path not found: {path:?}"))
    }

    fn all_workspace_members(&self, ignore: Option<&Path>) -> Vec<PathBuf> {
        self::WORKSPACE_MEMBERS
            .iter()
            .map(|member| self.workspace_path(member))
            .filter(|member| ignore.map_or(true, |ignore| !member.starts_with(ignore)))
            .collect()
    }

    pub fn build(self) -> Tasks {
        Tasks::new(self.tasks, self.workspace)
    }
}

fn parse_cmd(prepend: &str, string: &str) -> Vec<String> {
    let parts = string.split_whitespace().map(ToString::to_string).collect();
    if prepend.is_empty() {
        parts
    } else {
        [vec![prepend.to_owned()], parts].concat()
    }
}

include!(concat!(env!("OUT_DIR"), "/workspace_members.rs"));
