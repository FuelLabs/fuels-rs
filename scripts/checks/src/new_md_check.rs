use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, HashSet},
        fs::File,
        io::{BufRead, BufReader, Write},
        str::FromStr,
    };

    use itertools::Itertools;
    use pretty_assertions::assert_eq;
    use walkdir::WalkDir;

    use crate::md_check::Include;

    use super::*;

    #[derive(Debug, PartialEq)]
    struct Anchor {
        name: String,
        start_line: u64,
        end_line: u64,
    }

    struct Anchors {
        anchors: HashMap<PathBuf, Vec<Anchor>>,
    }

    impl Anchors {
        fn new() -> Self {
            Anchors {
                anchors: HashMap::new(),
            }
        }

        fn load(&mut self, file: &Path) -> Result<()> {
            let lines = std::fs::read_to_string(&file)?.lines().collect_vec();

            let mut start_anchors: Vec<(u64, String)> = vec![];
            let mut end_anchors: Vec<(u64, String)> = vec![];

            for (i, line) in lines.iter().enumerate() {
                if let Some(anchor) = line.strip_prefix("// ANCHOR: ") {
                    start_anchors.push((i as u64, anchor.to_string()));
                } else if let Some(anchor) = line.strip_prefix("// ANCHOR_END: ") {
                    end_anchors.push((i as u64, anchor.to_string()));
                }
            }

            Ok(())
        }
    }

    pub fn load_anchors(path: &[PathBuf]) -> Result<Anchors> {
        todo!()
    }

    #[test]
    fn parses_anchors() {
        // given
        let temp_dir = tempfile::tempdir().unwrap();
        let file = temp_dir.path().join("file.rs");
        let lines = [
            "// ANCHOR: outer",
            "// ANCHOR: inner",
            "CONTENT",
            "// ANCHOR_END: inner",
            "// ANCHOR_END: outer",
        ];

        write_to_file(&file, &lines).unwrap();

        let mut anchors = Anchors::new();

        // when
        anchors.load(&file).unwrap();

        // then
        assert_eq!(
            load_anchors,
            [
                Anchor {
                    name: "outer".to_string(),
                    start_line: 0,
                    end_line: 4,
                },
                Anchor {
                    name: "inner".to_string(),
                    start_line: 1,
                    end_line: 3,
                },
            ]
        )
    }

    #[derive(thiserror::Error, Debug, PartialEq)]
    enum Error {
        #[error("Missing anchor. Needed in {usage}, expected in {expected_file} with name {name}")]
        MissingAnchor {
            usage: PathBuf,
            expected_file: PathBuf,
            name: String,
        },
        #[error("{0}")]
        Other(String),
    }

    type Result<T> = std::result::Result<T, Error>;

    impl From<walkdir::Error> for Error {
        fn from(err: walkdir::Error) -> Self {
            Error::Other(err.to_string())
        }
    }
    impl From<anyhow::Error> for Error {
        fn from(err: anyhow::Error) -> Self {
            Error::Other(err.to_string())
        }
    }

    impl From<std::io::Error> for Error {
        fn from(err: std::io::Error) -> Self {
            Error::Other(err.to_string())
        }
    }

    #[test]
    fn missing_anchors_detected() {
        // given
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path();

        let anchors = path.join("includes.md");
        let lines = [r"{{#include ./test_anchor_data.rs:test_anchor_block_comment}}"].join("\n");
        std::fs::write(&anchors, lines);

        // when
        let err = doit(&anchors).expect_err("Expected error");

        // then
        let expected = Error::MissingAnchor {
            usage: path.join("includes.md"),
            expected_file: path.join("test_anchor_data.rs"),
            name: "test_anchor_block_comment".to_string(),
        };

        assert_eq!(err, expected);
    }

    fn doit(dir: &Path, excludes: &[PathBuf]) -> Result<()> {
        let code_files = find_files(dir, &["rs", "sw"], excludes)?;
        let anchors = load_anchors(&code_files)?;

        let md_files = find_files(dir, &["md"], excludes)?;
        let includes = extract_includes(&md_files)?;

        includes_valid(&includes, &anchors)?;
        no_unused_anchors(&anchors, &includes)?;

        Ok(())
    }

    fn no_unused_anchors(
        lookup: &HashMap<PathBuf, Vec<String>>,
        includes: &[Include],
    ) -> Result<()> {
        todo!()
    }

    fn includes_valid(includes: &[Include], lookup: &HashMap<PathBuf, Vec<String>>) -> Result<()> {
        todo!()
    }

    fn extract_includes(md_files: &[PathBuf]) -> Result<Vec<Include>> {
        todo!()
    }

    fn find_files(path: &Path, extensions: &[&str], excludes: &[PathBuf]) -> Result<Vec<PathBuf>> {
        let mut files = vec![];
        for entry in WalkDir::new(path) {
            let path = entry?.path().to_owned();
            if excludes.contains(&path) || !path.is_file() {
                continue;
            }

            if let Some(ext) = path.extension() {
                if extensions.contains(&ext.to_str().unwrap()) {
                    files.push(path);
                }
            }
        }
        Ok(files)
    }

    fn write_to_file(path: &Path, lines: &[&str]) -> anyhow::Result<()> {
        let mut file = File::create(path)?;

        for line in lines {
            writeln!(&mut file, "{line}")?;
        }

        file.sync_all()?;

        Ok(())
    }
}
