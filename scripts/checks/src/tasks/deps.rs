use std::collections::BTreeSet;

use super::short_sha256;

use super::task::Task;

use itertools::Itertools;
use serde::Serialize;
use serde::Serializer;

#[derive(Debug, Clone, serde::Serialize, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SwayArtifacts {
    TypePaths,
    Normal,
}

#[derive(Debug, Default, Clone, serde::Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RustDeps {
    pub nightly: bool,
    #[serde(serialize_with = "comma_separated")]
    pub components: BTreeSet<String>,
}

pub(crate) fn comma_separated<S>(
    components: &BTreeSet<String>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let components = components.iter().join(",");
    components.serialize(serializer)
}

#[derive(Debug, Default, Clone, serde::Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CargoDeps {
    pub hack: bool,
    pub nextest: bool,
    pub machete: bool,
    pub udeps: bool,
}

impl std::ops::Add for CargoDeps {
    type Output = Self;
    fn add(mut self, other: Self) -> Self {
        self += other;
        self
    }
}

impl std::ops::AddAssign for CargoDeps {
    fn add_assign(&mut self, other: Self) {
        self.hack |= other.hack;
        self.nextest |= other.nextest;
        self.machete |= other.machete;
    }
}

#[derive(Debug, Default, Clone, serde::Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CiDeps {
    pub fuel_core_binary: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust: Option<RustDeps>,
    pub wasm: bool,
    pub cargo: CargoDeps,
    pub typos_cli: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sway_artifacts: Option<SwayArtifacts>,
}

impl std::ops::Add for CiDeps {
    type Output = Self;
    fn add(mut self, other: Self) -> Self {
        self += other;
        self
    }
}

impl std::ops::AddAssign for CiDeps {
    fn add_assign(&mut self, other: Self) {
        self.fuel_core_binary |= other.fuel_core_binary;

        let rust = match (self.rust.take(), other.rust) {
            (Some(mut self_rust), Some(other_rust)) => {
                self_rust.nightly |= other_rust.nightly;
                self_rust.components = self_rust
                    .components
                    .union(&other_rust.components)
                    .cloned()
                    .collect();
                Some(self_rust)
            }
            (Some(self_rust), None) => Some(self_rust),
            (None, Some(other_rust)) => Some(other_rust),
            (None, None) => None,
        };
        self.rust = rust;

        self.wasm |= other.wasm;
        self.cargo += other.cargo;
        self.typos_cli |= other.typos_cli;

        let sway_artifacts = match (self.sway_artifacts, other.sway_artifacts) {
            (Some(self_sway), Some(other_sway)) => {
                if self_sway != other_sway {
                    panic!(
                        "Deps cannot be unified. Cannot have type paths and normal artifacts at once! {self_sway:?} != {other_sway:?}",
                    );
                }
                Some(self_sway)
            }
            (Some(self_sway), None) => Some(self_sway),
            (None, Some(other_sway)) => Some(other_sway),
            (None, None) => None,
        };
        self.sway_artifacts = sway_artifacts;
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CiJob {
    pub(crate) deps: CiDeps,
    // Comma separated task ids
    pub(crate) task_ids: String,
    pub(crate) name: String,
    // Must not contain commas, rust-cache complains
    pub(crate) cache_key: String,
}

impl CiJob {
    pub fn new(deps: CiDeps, tasks: &[&Task], name: String) -> Self {
        let ids = tasks.iter().map(|t| t.id()).join(",");
        Self {
            deps,
            cache_key: short_sha256(&ids),
            task_ids: ids,
            name,
        }
    }
}
