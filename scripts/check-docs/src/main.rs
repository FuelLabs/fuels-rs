mod test_anchor_data;

extern crate core;

use anyhow::{anyhow, bail, Error};
use itertools::{chain, Itertools};
use regex::Regex;
use std::path::{Path, PathBuf};

fn main() -> anyhow::Result<(), Error> {
    let text_w_anchors = search_for_patterns_in_project("ANCHOR")?;

    let (starts, ends) = extract_starts_and_ends(&text_w_anchors)?;

    let (valid_anchors, anchor_errors) = filter_valid_anchors(starts, ends);

    let text_mentioning_include = search_for_patterns_in_project("{{#include")?;
    let includes = parse_includes(text_mentioning_include)?;

    let (include_errors, additional_warnings) = validate_includes(includes, valid_anchors);

    report_warnings(&additional_warnings);

    if !anchor_errors.is_empty() || !include_errors.is_empty() {
        report_errors("anchors", &anchor_errors);
        report_errors("includes", &include_errors);
        bail!("Finished with errors");
    }
    Ok(())
}

fn report_errors(error_type: &str, errors: &[Error]) {
    eprintln!("Invalid {} detected!", error_type);
    for error in errors {
        eprintln!("{error}")
    }
}

fn report_warnings(warnings: &[Error]) {
    for warning in warnings {
        eprintln!("WARNING! {warning}")
    }
}

fn validate_includes(
    includes: Vec<Include>,
    valid_anchors: Vec<Anchor>,
) -> (Vec<Error>, Vec<Error>) {
    let (pairs, errors): (Vec<_>, Vec<_>) = includes
        .into_iter()
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
struct Include {
    anchor_name: String,
    anchor_file: PathBuf,
    include_file: PathBuf,
    line_no: usize,
}

fn parse_includes(text_w_includes: String) -> anyhow::Result<Vec<Include>, Error> {
    let apply_regex = |regex: Regex| {
        text_w_includes
            .lines()
            .filter_map(|line| regex.captures(line))
            .map(|capture| {
                let include_file = PathBuf::from(&capture[1]).canonicalize()?;
                let line_no = capture[2].parse()?;
                let anchor_file = PathBuf::from(&capture[3]);
                let anchor_name = capture[4].to_owned();

                let the_path = include_file.parent().unwrap().join(anchor_file);

                let anchor_file = the_path.canonicalize().unwrap_or_else(|err| {
                    panic!("{the_path:?} when canonicalized gives error {:?}", err)
                });

                Ok(Include {
                    anchor_name,
                    anchor_file,
                    include_file,
                    line_no,
                })
            })
            .collect::<Result<Vec<_>, Error>>()
    };

    apply_regex(Regex::new(
        r"^(\S+):(\d+):\s*\{\{\s*#include\s*(\S+)\s*:\s*(\S+)\s*\}\}",
    )?)
}

fn filter_valid_anchors(starts: Vec<Anchor>, ends: Vec<Anchor>) -> (Vec<Anchor>, Vec<Error>) {
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

fn filter_unused_ends<'a>(ends: &'a [Anchor], pairs: &[(Anchor, Anchor)]) -> Vec<&'a Anchor> {
    ends.iter()
        .filter(|end| {
            let end_used_in_pairs = pairs.iter().any(|(_, used_end)| *end == used_end);
            !end_used_in_pairs
        })
        .collect()
}

fn check_validity_of_anchor_pair(begin: &Anchor, end: &Anchor) -> Option<anyhow::Error> {
    if begin.line_no > end.line_no {
        Some(anyhow!("The end of the anchor appears before the beginning. End anchor: {end:?}. Begin anchor: {begin:?}"))
    } else {
        None
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct Anchor {
    line_no: usize,
    name: String,
    file: PathBuf,
}

#[allow(dead_code)]
enum TestEnum {
    Anchor(Vec<Anchor>),
    Include(Vec<Include>),
    Errors(Vec<Error>),
}

fn extract_starts_and_ends(
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
                    line_no: line_no.parse().unwrap(),
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

fn search_for_patterns_in_project(pattern: &str) -> anyhow::Result<String> {
    let grep_project = std::process::Command::new("grep")
        .arg("--binary-files=without-match")
        .arg("--with-filename")
        .arg("--dereference-recursive")
        .arg("--line-number")
        .arg("--exclude-dir=scripts")
        .arg(pattern)
        .arg(".")
        .output()
        .expect("failed grep command");

    if !grep_project.status.success() {
        bail!("Failed running grep command for searching {}", pattern);
    }

    Ok(String::from_utf8(grep_project.stdout)?)
}

#[cfg(test)]
mod tests {
    use crate::{
        extract_starts_and_ends, filter_valid_anchors, parse_includes, validate_includes, TestEnum,
    };
    use anyhow::bail;

    fn test_search_for_patterns(pattern: &str, test_file: &str) -> anyhow::Result<String> {
        let grep = std::process::Command::new("grep")
            .arg("--binary-files=without-match")
            .arg("--with-filename")
            .arg("--dereference-recursive")
            .arg("--line-number")
            .arg("--exclude-dir=scripts")
            .arg(pattern)
            .arg(test_file)
            .output()
            .expect("failed grep command");

        if !grep.status.success() {
            bail!("Failed running grep command for searching {}", pattern);
        }

        Ok(String::from_utf8(grep.stdout)?)
    }

    fn contains_any(vec: &TestEnum, str: &str) -> bool {
        match vec {
            TestEnum::Anchor(anchor_vec) => anchor_vec.iter().any(|anchor| anchor.name == str),
            TestEnum::Include(include_vec) => {
                include_vec.iter().any(|include| include.anchor_name == str)
            }
            TestEnum::Errors(err_vec) => err_vec.iter().any(|err| err.to_string().starts_with(str)),
        }
    }

    #[test]
    fn test_anchors() -> anyhow::Result<()> {
        let test_data = test_search_for_patterns("ANCHOR", "src/test_anchor_data.rs")?;

        let (starts, ends) = extract_starts_and_ends(&test_data)?;
        let (valid_anchors, anchor_errors) = filter_valid_anchors(starts, ends);

        let valid_vec = TestEnum::Anchor(valid_anchors.clone());
        let anchor_err_vec = TestEnum::Errors(anchor_errors);

        assert!(contains_any(&valid_vec, "test_anchor_line_comment"));
        assert!(contains_any(&valid_vec, "test_anchor_block_comment"));
        assert!(contains_any(&valid_vec, "test_with_more_forward_slashes"));
        assert!(!contains_any(&valid_vec, "no_anchor_with_this_name"));

        assert!(contains_any(
            &anchor_err_vec,
            "Missing anchor start for Anchor { line_no: 10, name: \"test_no_anchor_beginning\""
        ));
        assert!(contains_any(&anchor_err_vec, "Couldn't find a matching end anchor for Anchor { line_no: 12, name: \"test_no_anchor_end\""));
        assert!(contains_any(&anchor_err_vec, "The end of the anchor appears before the beginning. End anchor: Anchor { line_no: 14, name: \"test_end_before_beginning\""));

        assert!(contains_any(&anchor_err_vec, "Found too many matching anchor ends for anchor: Anchor { line_no: 17, name: \"test_same_name_multiple_time\""));
        assert!(contains_any(&anchor_err_vec, "Found too many matching anchor ends for anchor: Anchor { line_no: 20, name: \"test_same_name_multiple_time\""));
        // Caused by too many matching anchors
        assert!(contains_any(
            &anchor_err_vec,
            "Missing anchor start for Anchor { line_no: 18, name: \"test_same_name_multiple_time\""
        ));
        assert!(contains_any(
            &anchor_err_vec,
            "Missing anchor start for Anchor { line_no: 21, name: \"test_same_name_multiple_time\""
        ));

        let text_mentioning_include =
            test_search_for_patterns("{{#include", "src/test_include_data.md")?;

        let includes = parse_includes(text_mentioning_include)?;

        let includes_vec = TestEnum::Include(includes.clone());

        assert!(contains_any(&includes_vec, "test_anchor_line_comment"));
        assert!(contains_any(&includes_vec, "test_anchor_block_comment"));
        assert!(contains_any(
            &includes_vec,
            "test_with_more_forward_slashes"
        ));

        let (include_errors, _) = validate_includes(includes, valid_anchors);

        let include_err_vec = TestEnum::Errors(include_errors);

        assert!(contains_any(
            &include_err_vec,
            "No anchor available to satisfy include Include { anchor_name: \"no_existing_anchor\""
        ));

        Ok(())
    }
}
