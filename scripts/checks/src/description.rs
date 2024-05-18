use itertools::chain;
use itertools::Itertools;
use std::collections::HashSet;
use std::path::PathBuf;

use crate::task::Command;
use crate::task::Dependency;
use crate::task::Task;

pub fn ci(workspace: PathBuf, sway_type_paths: bool) -> HashSet<Task> {
    generate_tasks(workspace, sway_type_paths, |tb| tb.ci())
}

pub fn hack_features(workspace: PathBuf, sway_type_paths: bool) -> HashSet<Task> {
    generate_tasks(workspace, sway_type_paths, |tb| tb.hack_features())
}

pub fn hack_deps(workspace: PathBuf, sway_type_paths: bool) -> HashSet<Task> {
    generate_tasks(workspace, sway_type_paths, |tb| tb.hack_deps())
}

fn generate_tasks<F>(workspace: PathBuf, sway_type_paths: bool, task_fn: F) -> HashSet<Task>
where
    F: FnOnce(TasksBuilder) -> HashSet<Task> + Copy,
{
    let wo_type_paths = task_fn(TasksBuilder::new(workspace.clone(), false, &["-Dwarnings"]));

    if sway_type_paths {
        let w_type_paths = task_fn(TasksBuilder::new(workspace, true, &["-Dwarnings"]));
        w_type_paths.difference(&wo_type_paths).cloned().collect()
    } else {
        wo_type_paths
    }
}

struct TasksBuilder {
    workspace: PathBuf,
    sway_type_paths: bool,
    rust_flags: Vec<String>,
}

impl TasksBuilder {
    fn new(workspace: PathBuf, sway_type_paths: bool, rust_flags: &[&str]) -> Self {
        Self {
            workspace,
            sway_type_paths,
            rust_flags: rust_flags.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn ci(&self) -> HashSet<Task> {
        chain!(
            self.common(),
            self.e2e_specific(),
            self.wasm_specific(),
            self.workspace_level(),
        )
        .collect()
    }

    pub fn hack_features(&self) -> HashSet<Task> {
        chain!(self.hack_features_common(), self.hack_features_e2e()).collect()
    }

    pub fn hack_deps(&self) -> HashSet<Task> {
        self.hack_deps_common().collect()
    }

    fn workspace_path(&self, path: &str) -> PathBuf {
        let path = self.workspace.join(path);
        path.canonicalize()
            .unwrap_or_else(|_| panic!("Path not found: {:?}", path))
    }

    fn hack_features_common(&self) -> Vec<Task> {
        self.all_workspace_members()
            .into_iter()
            .flat_map(|member| {
                let commands = if !member.ends_with("e2e") {
                    let deps = [Dependency::RustStable, Dependency::CargoHack];
                    vec![
                        self.cargo("hack --feature-powerset check", &deps),
                        self.cargo("hack --feature-powerset check --tests", &deps),
                    ]
                } else {
                    vec![]
                };

                commands.into_iter().map(move |cmd| Task {
                    cwd: member.clone(),
                    cmd,
                })
            })
            .collect()
    }

    fn hack_deps_common(&self) -> impl Iterator<Item = Task> + '_ {
        self.all_workspace_members().into_iter().flat_map(|member| {
            let commands = if !member.ends_with("e2e") {
                let deps = [Dependency::RustNightly, Dependency::CargoHack];
                vec![
                    self.cargo("+nightly hack --deps udeps", &deps),
                    self.cargo("+nightly hack --deps udeps --tests", &deps),
                ]
            } else {
                vec![]
            };
            commands.into_iter().map(move |cmd| Task {
                cwd: member.clone(),
                cmd,
            })
        })
    }

    fn hack_features_e2e(&self) -> Vec<Task> {
        let exclude_features = if self.sway_type_paths {
            ""
        } else {
            "--exclude-features test-type-paths"
        };

        let deps = [Dependency::RustStable, Dependency::CargoHack];
        [
            self.cargo(
                format!("hack --feature-powerset {exclude_features} check"),
                &deps,
            ),
            self.cargo(
                format!("hack --feature-powerset {exclude_features} check --tests"),
                &deps,
            ),
        ]
        .map(|cmd| Task {
            cwd: self.workspace_path("e2e"),
            cmd,
        })
        .to_vec()
    }

    fn cargo(&self, cmd: impl Into<String>, deps: &[Dependency]) -> Command {
        self.cargo_full(cmd, None, deps)
    }

    fn cargo_full(
        &self,
        cmd: impl Into<String>,
        env: Option<(&str, &str)>,
        deps: &[Dependency],
    ) -> Command {
        let mut envs = self.rust_flags_env();

        if let Some(env) = env {
            envs.push((env.0.into(), env.1.into()));
        }

        Command::Custom {
            program: "cargo".to_string(),
            args: parse_cmd("", &cmd.into()),
            env: envs,
            deps: deps.to_vec(),
        }
    }

    fn rust_flags_env(&self) -> Vec<(String, String)> {
        let value = self.rust_flags.iter().join(" ");
        vec![("RUSTFLAGS".to_owned(), value)]
    }

    fn custom(&self, program: &str, args: &str, deps: &[Dependency]) -> Command {
        Command::Custom {
            program: program.to_owned(),
            args: parse_cmd("", args),
            env: vec![],
            deps: deps.to_vec(),
        }
    }

    fn e2e_specific(&self) -> impl Iterator<Item = Task> + '_ {
        let type_paths_feat = if self.sway_type_paths {
            ",test-type-paths"
        } else {
            ""
        };

        let sway_artifacts_dep = Dependency::SwayArtifacts {
            type_paths: self.sway_type_paths,
        };
        let test_deps = [
            Dependency::RustStable,
            Dependency::Nextest,
            sway_artifacts_dep,
        ];
        let clippy_deps = [
            Dependency::RustStable,
            Dependency::Clippy,
            sway_artifacts_dep,
        ];
        [
            self.cargo(
                format!("nextest run --features default,fuel-core-lib{type_paths_feat}"),
                &test_deps,
            ),
            self.cargo(
                format!("nextest run --features default,{type_paths_feat}"),
                &[test_deps.to_vec(), vec![Dependency::FuelCoreBinary]].concat(),
            ),
            self.cargo(
                format!("clippy --all-targets --no-deps --features default,{type_paths_feat}"),
                &clippy_deps,
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
                "wasm-pack test --node",
                "",
                &[Dependency::Wasm, Dependency::RustStable],
            ),
        })
    }

    fn workspace_level(&self) -> impl Iterator<Item = Task> {
        [
            Command::MdCheck,
            self.custom(
                "cargo-machete --skip-target-dir",
                "",
                &[Dependency::CargoMachete],
            ),
            self.cargo(
                "clippy --workspace --all-features",
                &[Dependency::RustStable, Dependency::Clippy],
            ),
            self.custom("typos", "", &[Dependency::TyposCli]),
        ]
        .map(|cmd| Task {
            cwd: self.workspace_path("."),
            cmd,
        })
        .into_iter()
    }

    fn common(&self) -> impl Iterator<Item = Task> + '_ {
        self.all_workspace_members().into_iter().flat_map(|member| {
            let mut commands = vec![
                self.cargo(
                    "fmt --verbose --check",
                    &[Dependency::RustStable, Dependency::RustFmt],
                ),
                self.custom("typos", "", &[Dependency::TyposCli]),
            ];
            // e2e ignored because we have to control the features carefully (e.g. rocksdb, test-type-paths, etc)
            if !member.ends_with("e2e") {
                let cmd = self.cargo(
                    "clippy --all-targets --all-features --no-deps",
                    &[Dependency::RustStable, Dependency::Clippy],
                );
                commands.push(cmd);
            }

            // e2e ignored because we have to control the features carefully (e.g. rocksdb, test-type-paths, etc)
            // wasm ignored because wasm tests need to be run with wasm-pack
            if !member.ends_with("wasm-tests") && !member.ends_with("e2e") {
                let cmd = self.cargo(
                    "nextest run --all-features",
                    &[Dependency::RustStable, Dependency::Nextest],
                );
                commands.push(cmd);
            }

            // because these don't have libs
            if !member.ends_with("e2e")
                && !member.ends_with("scripts/checks")
                && !member.ends_with("wasm-tests")
            {
                let cmd = self.cargo("test --doc", &[Dependency::RustStable]);
                commands.push(cmd);

                let cmd = self.cargo_full(
                    "doc --document-private-items",
                    Some(("RUSTDOCFLAGS", "-Dwarnings")),
                    &[Dependency::RustStable],
                );
                commands.push(cmd);
            }
            commands.into_iter().map(move |cmd| Task {
                cwd: member.clone(),
                cmd,
            })
        })
    }

    fn all_workspace_members(&self) -> Vec<PathBuf> {
        self::WORKSPACE_MEMBERS
            .iter()
            .map(|member| self.workspace_path(member))
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
