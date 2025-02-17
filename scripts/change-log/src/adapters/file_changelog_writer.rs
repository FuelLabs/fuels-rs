use crate::ports::changelog_writer::ChangelogWriter;
use std::fs::File;
use std::io::Write;

pub struct FileChangelogWriter {
    pub file_path: String,
}

impl ChangelogWriter for FileChangelogWriter {
    fn write_changelog(&self, changelog: &str) -> std::io::Result<()> {
        let mut file = File::create(&self.file_path)?;
        file.write_all(changelog.as_bytes())
    }
}
