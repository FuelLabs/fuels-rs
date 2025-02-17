#[derive(Debug, Clone)]
pub struct ChangelogInfo {
    pub is_breaking: bool,
    pub pr_type: String,
    pub bullet_point: String,
    pub migration_note: String,
    pub release_notes: String,
    // These fields are kept for debugging or traceability.
    pub pr_number: u64,
    pub pr_title: String,
    pub pr_author: String,
    pub pr_url: String,
}
