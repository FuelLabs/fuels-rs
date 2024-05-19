use itertools::chain;
use itertools::Itertools;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use crate::task::CargoDeps;
use crate::task::CiDeps;
use crate::task::Command;
use crate::task::RustDeps;
use crate::task::SwayArtifacts;
use crate::task::Task;
use crate::task::Tasks;

pub fn normal(workspace_root: PathBuf) -> Tasks {
    let builder = TasksBuilder::new(workspace_root.clone(), &["-Dwarnings"]);
    Tasks::new(builder.local(), workspace_root)
}

pub fn hack_features(workspace_root: PathBuf) -> Tasks {
    let builder = TasksBuilder::new(workspace_root.clone(), &["-Dwarnings"]);

    Tasks::new(builder.hack_features(), workspace_root)
}

pub fn hack_deps(workspace_root: PathBuf) -> Tasks {
    let builder = TasksBuilder::new(workspace_root.clone(), &["-Dwarnings"]);
    Tasks::new(builder.hack_deps(), workspace_root)
}

struct TasksBuilder {
    workspace: PathBuf,
    rust_flags: Vec<String>,
}

impl TasksBuilder {
    fn new(workspace: PathBuf, rust_flags: &[&str]) -> Self {
        Self {
            workspace,
            rust_flags: rust_flags.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn local(&self) -> BTreeSet<Task> {
        chain!(
            self.common(),
            self.e2e_specific(),
            self.wasm_specific(),
            self.workspace_level(),
        )
        .collect()
    }

    pub fn hack_features(&self) -> BTreeSet<Task> {
        chain!(self.hack_features_common(), self.hack_features_e2e()).collect()
    }

    pub fn hack_deps(&self) -> BTreeSet<Task> {
        self.hack_deps_common().collect()
    }

    fn common(&self) -> impl Iterator<Item = Task> + '_ {
        self.all_workspace_members(None)
            .into_iter()
            .flat_map(|member| {
                let deps = {
                    // Some examples run abigen! on sway projects in e2e
                    let sway_artifacts = member
                        .starts_with(self.workspace_path("examples"))
                        .then_some(SwayArtifacts::Normal);

                    CiDeps {
                        sway_artifacts,
                        ..Default::default()
                    }
                };

                let mut commands = vec![
                    self.cargo_fmt("--verbose --check", deps.clone()),
                    self.custom(
                        "typos",
                        "",
                        &CiDeps {
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
                        deps.clone(),
                    );
                    commands.push(cmd);
                }

                commands.into_iter().map(move |cmd| Task {
                    cwd: member.clone(),
                    cmd,
                })
            })
    }

    fn e2e_specific(&self) -> impl Iterator<Item = Task> + '_ {
        [
            self.cargo_nextest(
                "run --features default,fuel-core-lib,test-type-paths",
                CiDeps {
                    sway_artifacts: Some(SwayArtifacts::TypePaths),
                    ..Default::default()
                },
            ),
            self.cargo_nextest(
                "run --features default,fuel-core-lib",
                CiDeps {
                    sway_artifacts: Some(SwayArtifacts::Normal),
                    ..Default::default()
                },
            ),
            self.cargo_nextest(
                "run --features default,test-type-paths",
                CiDeps {
                    fuel_core_binary: true,
                    sway_artifacts: Some(SwayArtifacts::Normal),
                    ..Default::default()
                },
            ),
            self.cargo_clippy(
                "--all-targets --no-deps --features default,test-type-paths",
                CiDeps {
                    sway_artifacts: Some(SwayArtifacts::TypePaths),
                    ..Default::default()
                },
            ),
        ]
        .map(|cmd| Task {
            cwd: self.workspace_path("e2e"),
            cmd,
        })
        .into_iter()
    }

    fn wasm_specific(&self) -> impl Iterator<Item = Task> {
        std::iter::once(Task {
            cwd: self.workspace_path("wasm-tests"),
            cmd: self.custom(
                "wasm-pack",
                "test --node",
                &CiDeps {
                    wasm: true,
                    ..Default::default()
                },
            ),
        })
    }

    fn workspace_level(&self) -> impl Iterator<Item = Task> {
        [
            Command::MdCheck,
            self.custom(
                "cargo-machete",
                "--skip-target-dir",
                &CiDeps {
                    cargo: CargoDeps {
                        machete: true,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ),
            self.cargo_clippy(
                "--workspace --all-features",
                CiDeps {
                    sway_artifacts: Some(SwayArtifacts::Normal),
                    ..Default::default()
                },
            ),
            self.custom(
                "typos",
                "",
                &CiDeps {
                    typos_cli: true,
                    ..Default::default()
                },
            ),
        ]
        .map(|cmd| Task {
            cwd: self.workspace_path("."),
            cmd,
        })
        .into_iter()
    }

    fn hack_features_common(&self) -> Vec<Task> {
        let ignore = self.workspace_path("e2e");
        self.all_workspace_members(Some(&ignore))
            .into_iter()
            .flat_map(|member| {
                [
                    self.cargo_hack("--feature-powerset check", CiDeps::default()),
                    self.cargo_hack("--feature-powerset check --tests", CiDeps::default()),
                ]
                .into_iter()
                .map(move |cmd| Task {
                    cwd: member.clone(),
                    cmd,
                })
            })
            .collect()
    }

    fn hack_features_e2e(&self) -> Vec<Task> {
        [
            self.cargo_hack(
                "--feature-powerset check --tests",
                CiDeps {
                    sway_artifacts: Some(SwayArtifacts::TypePaths),
                    ..Default::default()
                },
            ),
            self.cargo_hack(
                "--feature-powerset --exclude-features test-type-paths check --tests",
                CiDeps {
                    sway_artifacts: Some(SwayArtifacts::Normal),
                    ..Default::default()
                },
            ),
        ]
        .map(|cmd| Task {
            cwd: self.workspace_path("e2e"),
            cmd,
        })
        .to_vec()
    }

    fn hack_deps_common(&self) -> impl Iterator<Item = Task> + '_ {
        let ignore = self.workspace_path("e2e");
        self.all_workspace_members(Some(&ignore))
            .into_iter()
            .flat_map(|member| {
                let deps = CiDeps {
                    cargo: CargoDeps {
                        udeps: true,
                        ..Default::default()
                    },
                    rust: Some(RustDeps {
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
    }

    fn cargo_fmt(&self, cmd: impl Into<String>, mut deps: CiDeps) -> Command {
        deps += CiDeps {
            rust: Some(RustDeps {
                components: BTreeSet::from_iter(["rustfmt".to_string()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let cmd = format!("fmt {}", cmd.into());

        self.cargo(cmd, None, deps)
    }

    fn cargo_clippy(&self, cmd: impl Into<String>, mut deps: CiDeps) -> Command {
        deps += CiDeps {
            rust: Some(RustDeps {
                components: BTreeSet::from_iter(["clippy".to_string()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let cmd = format!("clippy {}", cmd.into());
        self.cargo(cmd, None, deps)
    }

    fn cargo_hack(&self, cmd: impl Into<String>, mut deps: CiDeps) -> Command {
        deps += CiDeps {
            cargo: CargoDeps {
                hack: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let cmd = format!("hack {}", cmd.into());
        self.cargo(cmd, None, deps)
    }

    fn cargo_nextest(&self, cmd: impl Into<String>, mut deps: CiDeps) -> Command {
        deps += CiDeps {
            cargo: CargoDeps {
                nextest: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let cmd = format!("nextest {}", cmd.into());

        self.cargo(cmd, None, deps)
    }

    fn cargo(&self, cmd: impl Into<String>, env: Option<(&str, &str)>, deps: CiDeps) -> Command {
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
            deps: deps.clone(),
        }
    }

    fn custom(&self, program: &str, args: &str, deps: &CiDeps) -> Command {
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
            .unwrap_or_else(|_| panic!("Path not found: {:?}", path))
    }

    fn all_workspace_members(&self, ignore: Option<&Path>) -> Vec<PathBuf> {
        self::WORKSPACE_MEMBERS
            .iter()
            .map(|member| self.workspace_path(member))
            .filter(|member| ignore.map_or(true, |ignore| !member.starts_with(ignore)))
            .collect()
    }
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
