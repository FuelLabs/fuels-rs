#[derive(Debug, Clone)]
pub struct ChangelogInfo {
    pub is_breaking: bool,
    pub pr_type: String,
    pub bullet_point: String,
    pub migration_note: String,
    pub release_notes: String,
}
