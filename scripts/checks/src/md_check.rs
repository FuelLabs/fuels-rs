use anyhow::anyhow;
use anyhow::{bail, Error};
use duct::cmd;
use itertools::{chain, Itertools};
use regex::Regex;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

pub fn run(dir: &Path) -> anyhow::Result<(), Error> {
    let sources = ["packages", "e2e", "examples"].map(|source| dir.join(source));
    let text_w_anchors = search_for_pattern("ANCHOR", &sources)?;
    let (starts, ends) = extract_starts_and_ends(&text_w_anchors)?;
    let (valid_anchors, anchor_errors) = filter_valid_anchors(starts, ends);

    let text_mentioning_include = search_for_pattern("{{#include", &[dir.join("docs")])?;
    let (includes, include_path_errors) = parse_includes(text_mentioning_include);
    let (include_errors, additional_warnings) = validate_includes(includes, valid_anchors);

    let text_with_md_files = search_for_pattern(".md", &[dir.join("./docs/src/SUMMARY.md")])?;
    let md_files_in_summary = parse_md_files(text_with_md_files, dir.join("./docs/src/"));
    let md_files_in_src = find_files("*.md", dir.join("./docs/src/"), "SUMMARY.md")?;
    let md_files_errors = validate_md_files(md_files_in_summary, md_files_in_src);

    let errors = chain!(
        additional_warnings,
        anchor_errors,
        include_path_errors,
        include_errors,
        md_files_errors
    )
    .collect_vec();

    if !errors.is_empty() {
        let err_str = errors.iter().map(|err| err.to_string()).join("\n");
        bail!("Errors: {err_str}")
    }

    Ok(())
}

pub fn validate_includes(
    includes: Vec<Include>,
    valid_anchors: Vec<Anchor>,
) -> (Vec<Error>, Vec<Error>) {
    let (pairs, errors): (Vec<_>, Vec<_>) = includes
        .into_iter()
        .filter(|include| !include.anchor_name.is_empty())
        .map(|include| {
            let mut maybe_anchor = valid_anchors.iter().find(|anchor| {
                anchor.file == include.anchor_file && anchor.name == include.anchor_name
            });

            match maybe_anchor.take() {
                Some(anchor) => Ok(anchor.clone()),
                None => Err(anyhow!(
                    "No anchor available to satisfy include {include:?}"
                )),
            }
        })
        .partition_result();

    let additional_warnings = valid_anchors
        .iter()
        .filter(|valid_anchor| {
            let anchor_used_in_a_pair = pairs.iter().any(|anchor| anchor == *valid_anchor);
            !anchor_used_in_a_pair
        })
        .map(|unused_anchor| anyhow!("Anchor unused: {unused_anchor:?}!"))
        .collect::<Vec<_>>();

    (errors, additional_warnings)
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Include {
    pub anchor_name: String,
    pub anchor_file: PathBuf,
    pub include_file: PathBuf,
    pub line_no: usize,
}

pub fn parse_includes(text_w_includes: String) -> (Vec<Include>, Vec<Error>) {
    let apply_regex = |regex: Regex| {
        let (includes, errors): (Vec<_>, Vec<_>) = text_w_includes
            .lines()
            .filter_map(|line| regex.captures(line))
            .map(|capture| {
                let include_file = PathBuf::from(&capture[1]).canonicalize()?;
                let line_no = capture[2].parse()?;
                let anchor_file = PathBuf::from(&capture[3]);
                let anchor_name = capture.get(4).map_or("", |m| m.as_str()).to_string();

                let the_path = include_file.parent().unwrap().join(anchor_file);

                let anchor_file = the_path.canonicalize().map_err(|err| {
                    anyhow!(
                        "{the_path:?} when canonicalized gives error {err:?}\ninclude_file: {:?}",
                        include_file
                    )
                })?;

                Ok(Include {
                    anchor_name,
                    anchor_file,
                    include_file,
                    line_no,
                })
            })
            .partition_result();
        (includes, errors)
    };

    apply_regex(
        Regex::new(r"^(\S+):(\d+):\s*\{\{\s*#include\s*(\S+?)\s*(?::\s*(\S+)\s*)?\}\}")
            .expect("could not construct regex"),
    )
}

pub fn filter_valid_anchors(starts: Vec<Anchor>, ends: Vec<Anchor>) -> (Vec<Anchor>, Vec<Error>) {
    let find_anchor_end_by_name = |anchor_name: &str, file: &Path| {
        ends.iter()
            .filter(|el| el.name == *anchor_name && el.file == file)
            .collect::<Vec<_>>()
    };

    let (pairs, errors):(Vec<_>, Vec<_>) = starts.into_iter().map(|start| {
        let matches_by_name = find_anchor_end_by_name(&start.name, &start.file);

        let (begin, end) = match matches_by_name.as_slice() {
            [single_match] => Ok((start, (*single_match).clone())),
            [] => Err(anyhow!("Couldn't find a matching end anchor for {start:?}")),
            multiple_ends => Err(anyhow!("Found too many matching anchor ends for anchor: {start:?}. The matching ends are: {multiple_ends:?}")),
        }?;

        match check_validity_of_anchor_pair(&begin, &end) {
            None => Ok((begin, end)),
            Some(err) => {
                let err_msg = err.to_string();
                Err(anyhow!("{err_msg}"))
            }
        }
    }).partition_result();

    let additional_errors = filter_unused_ends(&ends, &pairs)
        .into_iter()
        .map(|unused_end| anyhow!("Missing anchor start for {unused_end:?}"))
        .collect::<Vec<_>>();

    let start_only = pairs.into_iter().map(|(begin, _)| begin).collect();

    (start_only, chain!(errors, additional_errors).collect())
}

pub fn filter_unused_ends<'a>(ends: &'a [Anchor], pairs: &[(Anchor, Anchor)]) -> Vec<&'a Anchor> {
    ends.iter()
        .filter(|end| {
            let end_used_in_pairs = pairs.iter().any(|(_, used_end)| *end == used_end);
            !end_used_in_pairs
        })
        .collect()
}

pub fn check_validity_of_anchor_pair(begin: &Anchor, end: &Anchor) -> Option<anyhow::Error> {
    if begin.line_no > end.line_no {
        Some(anyhow!("The end of the anchor appears before the beginning. End anchor: {end:?}. Begin anchor: {begin:?}"))
    } else {
        None
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Anchor {
    pub line_no: usize,
    pub name: String,
    pub file: PathBuf,
}

pub fn extract_starts_and_ends(
    text_w_anchors: &str,
) -> anyhow::Result<(Vec<Anchor>, Vec<Anchor>), Error> {
    let apply_regex = |regex: Regex| {
        text_w_anchors
            .lines()
            .filter_map(|line| regex.captures(line))
            .map(|capture| {
                let file = PathBuf::from(&capture[1]).canonicalize()?;
                let line_no = &capture[2];
                let anchor_name = &capture[3];

                Ok(Anchor {
                    line_no: line_no.parse()?,
                    name: anchor_name.to_string(),
                    file,
                })
            })
            .collect::<Result<Vec<_>, Error>>()
    };

    let begins = apply_regex(Regex::new(
        r"^(.+):(\d+):\s*(?:/{2,}|/\*)\s*ANCHOR\s*:\s*([\w_-]+)\s*(?:\*/)?",
    )?)?;
    let ends = apply_regex(Regex::new(
        r"^(.+):(\d+):\s*(?:/{2,}|/\*)\s*ANCHOR_END\s*:\s*([\w_-]+)\s*(?:\*/)?",
    )?)?;

    Ok((begins, ends))
}

pub fn parse_md_files(text_w_files: String, path: impl AsRef<Path>) -> HashSet<PathBuf> {
    let regex = Regex::new(r"\((.*\.md)\)").expect("could not construct regex");

    text_w_files
        .lines()
        .filter_map(|line| regex.captures(line))
        .map(|capture| {
            PathBuf::from(path.as_ref())
                .join(&capture[1])
                .canonicalize()
                .expect("could not canonicalize md path")
        })
        .collect()
}

pub fn validate_md_files(
    md_files_summary: HashSet<PathBuf>,
    md_files_in_src: String,
) -> Vec<Error> {
    md_files_in_src
        .lines()
        .filter_map(|file| {
            let file = PathBuf::from(file)
                .canonicalize()
                .expect("could not canonicalize md path");

            (!md_files_summary.contains(&file))
                .then(|| anyhow!("file `{}` not in SUMMARY.md", file.to_str().unwrap()))
        })
        .collect()
}

pub fn search_for_pattern(pattern: &str, location: &[PathBuf]) -> anyhow::Result<String> {
    let mut args = vec!["-H", "-n", "-r", "--binary-files=without-match", pattern];
    args.extend(location.iter().map(|path| path.to_str().unwrap()));

    duct::cmd("grep", args)
        .stdin_null()
        .stderr_null()
        .read()
        .map_err(|err| anyhow!("Failed running `grep` command for pattern '{pattern}': {err}"))
}

pub fn find_files(
    pattern: &str,
    location: impl AsRef<Path>,
    exclude: &str,
) -> anyhow::Result<String> {
    Ok(cmd!(
        "find",
        location.as_ref().to_str().unwrap(),
        "-type",
        "f",
        "-name",
        pattern,
        "!",
        "-name",
        exclude,
    )
    .stdin_null()
    .stderr_null()
    .read()?)
}

#[cfg(test)]
mod tests {

    use super::*;

    use anyhow::Error;

    enum TestEnum {
        Anchor(Vec<Anchor>),
        Include(Vec<Include>),
        Errors(Vec<Error>),
    }

    fn contains_any(vec: &TestEnum, str: &str) -> bool {
        match vec {
            TestEnum::Anchor(anchor_vec) => anchor_vec.iter().any(|anchor| anchor.name == str),
            TestEnum::Include(include_vec) => {
                include_vec.iter().any(|include| include.anchor_name == str)
            }
            TestEnum::Errors(err_vec) => err_vec.iter().any(|err| err.to_string().contains(str)),
        }
    }

    #[test]
    fn test_anchors() -> anyhow::Result<()> {
        let test_data = generate_test_data()?;
        let path = test_data.path();

        let data = search_for_pattern("ANCHOR", &[path.to_owned()])?;

        let (starts, ends) = extract_starts_and_ends(&data)?;
        let (valid_anchors, anchor_errors) = filter_valid_anchors(starts, ends);

        let valid_vec = TestEnum::Anchor(valid_anchors.clone());
        let anchor_err_vec = TestEnum::Errors(anchor_errors);

        assert!(contains_any(&valid_vec, "test_anchor_line_comment"));
        assert!(contains_any(&valid_vec, "test_anchor_block_comment"));
        assert!(contains_any(&valid_vec, "test_with_more_forward_slashes"));
        assert!(!contains_any(&valid_vec, "no_anchor_with_this_name"));

        assert!(contains_any(
            &anchor_err_vec,
            "Missing anchor start for Anchor { line_no: 11, name: \"test_no_anchor_beginning\""
        ));
        assert!(contains_any(&anchor_err_vec, "Couldn't find a matching end anchor for Anchor { line_no: 13, name: \"test_no_anchor_end\""));
        assert!(contains_any(&anchor_err_vec, "The end of the anchor appears before the beginning. End anchor: Anchor { line_no: 15, name: \"test_end_before_beginning\""));

        assert!(contains_any(&anchor_err_vec, "Found too many matching anchor ends for anchor: Anchor { line_no: 18, name: \"test_same_name_multiple_time\""));
        assert!(contains_any(&anchor_err_vec, "Found too many matching anchor ends for anchor: Anchor { line_no: 21, name: \"test_same_name_multiple_time\""));
        // Caused by too many matching anchors
        assert!(contains_any(
            &anchor_err_vec,
            "Missing anchor start for Anchor { line_no: 19, name: \"test_same_name_multiple_time\""
        ));
        assert!(contains_any(
            &anchor_err_vec,
            "Missing anchor start for Anchor { line_no: 22, name: \"test_same_name_multiple_time\""
        ));

        let text_mentioning_include = search_for_pattern("{{#include", &[path.to_owned()])?;

        let (includes, include_path_errors) = parse_includes(text_mentioning_include);

        let includes_vec = TestEnum::Include(includes.clone());

        assert!(contains_any(&includes_vec, "test_anchor_line_comment"));
        assert!(contains_any(&includes_vec, "test_anchor_block_comment"));
        assert!(contains_any(
            &includes_vec,
            "test_with_more_forward_slashes"
        ));
        assert!(contains_any(&includes_vec, "")); //Check the file include without anchor

        let include_path_errors = TestEnum::Errors(include_path_errors);

        assert!(contains_any(
            &include_path_errors,
            "test_anchor_data2.rs\" when canonicalized gives error Os { code: 2, kind: NotFound"
        ));

        assert!(contains_any(
            &include_path_errors,
            "test_anchor_data3.rs\" when canonicalized gives error Os { code: 2, kind: NotFound"
        ));

        let (include_errors, _) = validate_includes(includes, valid_anchors);

        let include_err_vec = TestEnum::Errors(include_errors);

        assert!(contains_any(
            &include_err_vec,
            "No anchor available to satisfy include Include { anchor_name: \"no_existing_anchor\""
        ));

        Ok(())
    }

    #[test]
    fn test_unused_md() -> anyhow::Result<()> {
        let test_data = generate_test_data()?;
        let path = test_data.path();

        let text_with_md_files = search_for_pattern(".md", &[path.join("docs/src/SUMMARY.md")])?;
        let md_files_in_summary = parse_md_files(text_with_md_files, path.join("docs/src/"));
        let md_files_in_src = find_files("*.md", path.join("docs/src/"), "SUMMARY.md")?;
        let md_files_errors = validate_md_files(md_files_in_summary, md_files_in_src);

        let error_msg = md_files_errors.first().unwrap().to_string();

        eprintln!("{error_msg}");
        assert!(error_msg.contains("test-not-there.md` not in SUMMARY.md"));

        Ok(())
    }

    fn generate_test_data() -> anyhow::Result<tempfile::TempDir> {
        let temp_dir = tempfile::tempdir()?;

        let anchor_data = r#"
// ANCHOR: test_anchor_line_comment
///// ANCHOR_END: test_anchor_line_comment

/* ANCHOR: test_anchor_block_comment */
/* ANCHOR_END: test_anchor_block_comment */

// ANCHOR: test_with_more_forward_slashes
///// ANCHOR_END: test_with_more_forward_slashes

// ANCHOR_END: test_no_anchor_beginning

// ANCHOR: test_no_anchor_end

// ANCHOR_END: test_end_before_beginning
// ANCHOR: test_end_before_beginning

// ANCHOR: test_same_name_multiple_time
// ANCHOR_END: test_same_name_multiple_time

// ANCHOR: test_same_name_multiple_time
// ANCHOR_END: test_same_name_multiple_time
"#;
        let path = temp_dir.path();
        std::fs::write(path.join("test_anchor_data.rs"), anchor_data)?;

        let include_data = r#"
```rust,ignore
{{#include ./test_anchor_data.rs:test_anchor_line_comment}}
```

```rust,ignore
{{#include ./test_anchor_data.rs:test_anchor_block_comment}}
```

```rust,ignore
{{#include ./test_anchor_data.rs:test_with_more_forward_slashes}}
```

```rust,ignore
{{#include ./test_anchor_data.rs:no_existing_anchor}}
```

Include file with correct path

```rust,ignore
{{#include ./test_anchor_data.rs}}
```

Include file with wrong path

```rust,ignore
{{#include ./test_anchor_data2.rs}}
```

Another include file with wrong path

```rust,ignore
{{#include ./test_anchor_data3.rs}}
```
"#;

        std::fs::write(path.join("test_include_data.md"), include_data)?;

        let src = path.join("docs/src");
        std::fs::create_dir_all(&src)?;

        let summary = r#"- [Test](./test.md)"#;
        std::fs::write(src.join("SUMMARY.md"), summary)?;

        std::fs::write(src.join("test.md"), "")?;
        std::fs::write(src.join("test-not-there.md"), "")?;

        Ok(temp_dir)
    }
}
