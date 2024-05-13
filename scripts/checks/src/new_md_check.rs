// Unused anchors
// include references non existent file
// include references non existent anchor
// anchor has no end
// anchor has multiple ends
// anchor has multiple beginnings
// md files not referenced in summary
//

use std::path::{Path, PathBuf};

fn find_files_by_extension(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = vec![];
    for entry in dir.read_dir()? {
        let path = entry?.path().canonicalize()?;
        if path.is_file() {
            files.push(path);
        } else {
            files.append(&mut find_files_by_extension(&path)?);
        }
    }

    Ok(files)
}

trait FileFinder {
    fn find_files(&self, extension: &str) -> anyhow::Result<Vec<PathBuf>>;
}

struct MdCheck {
    ignore: Vec<PathBuf>,
}

impl MdCheck {
    pub fn check(root: &Path) -> anyhow::Result<()> {
        // find all rs and sw files
        // load anchors text from them, check validity
        // parse that into anchors, note errors

        // find all md files
        // load includes from them
        // check if files referenced in includes exist
        // check if anchors referenced in includes exist
        // check if all anchors are used

        // check if all files are referenced in summary
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {}
}
