use itertools::Itertools;

use super::{deps, task::Task};

#[derive(Debug, Clone, serde::Serialize)]
pub struct CiJob {
    deps: deps::Deps,
    // Comma separated task ids
    task_ids: String,
    name: String,
    // Must not contain commas, rust-cache complains
    cache_key: String,
}

impl CiJob {
    pub fn new(deps: deps::Deps, tasks: &[&Task], name: String) -> Self {
        let ids = tasks.iter().map(|t| t.id()).join(",");
        Self {
            deps,
            cache_key: super::short_sha256(&ids),
            task_ids: ids,
            name,
        }
    }
}
