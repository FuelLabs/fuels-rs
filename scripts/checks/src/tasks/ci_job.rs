#[derive(Debug, Clone, serde::Serialize)]
pub struct CiJob {
    pub(crate) deps: deps::All,
    // Comma separated task ids
    pub(crate) task_ids: String,
    pub(crate) name: String,
    // Must not contain commas, rust-cache complains
    pub(crate) cache_key: String,
}

impl CiJob {
    pub fn new(deps: deps::All, tasks: &[&Task], name: String) -> Self {
        let ids = tasks.iter().map(|t| t.id()).join(",");
        Self {
            deps,
            cache_key: short_sha256(&ids),
            task_ids: ids,
            name,
        }
    }
}
