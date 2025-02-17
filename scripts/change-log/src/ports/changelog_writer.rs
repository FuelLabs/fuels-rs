/// This port abstracts the output of the changelog.
pub trait ChangelogWriter {
    /// Write the changelog content. For example, this could write to a file.
    fn write_changelog(&self, changelog: &str) -> std::io::Result<()>;
}
