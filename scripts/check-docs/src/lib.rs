use anyhow::{anyhow, bail, Error};
use itertools::{chain, Itertools};
use regex::Regex;
use std::path::{Path, PathBuf};

pub fn report_errors(error_type: &str, errors: &[Error]) {
    eprintln!("Invalid {} detected!", error_type);
    for error in errors {
        eprintln!("{error}")
    }
}

pub fn report_warnings(warnings: &[Error]) {
    for warning in warnings {
        eprintln!("WARNING! {warning}")
    }
}

pub fn validate_includes(
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
pub struct Include {
    pub anchor_name: String,
    pub anchor_file: PathBuf,
    pub include_file: PathBuf,
    pub line_no: usize,
}

pub fn parse_includes(text_w_includes: String) -> anyhow::Result<Vec<Include>, Error> {
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

pub fn search_for_patterns_in_project(pattern: &str) -> anyhow::Result<String> {
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
