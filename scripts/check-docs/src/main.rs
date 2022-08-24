extern crate core;

use anyhow::{anyhow, bail, Error};
use itertools::{chain, Itertools};
use regex::Regex;
use std::path::{Path, PathBuf};

fn main() {
    // let text_w_anchors = search_for_anchors_in_docs();
    let text_w_anchors = search_for_patterns_in_project("ANCHOR").unwrap();

    let (starts, ends) = extract_starts_and_ends(&text_w_anchors);

    let (valid_anchors, anchor_errors) = filter_valid_anchors(starts, ends);

    report_invalid_anchors(&anchor_errors);

    let text_mentioning_include = search_for_patterns_in_project("{{#include").unwrap();
    let includes = parse_includes(text_mentioning_include);

    let include_errors = validate_includes(includes, valid_anchors);

    report_invalid_includes(&include_errors);

    if !anchor_errors.is_empty() || !include_errors.is_empty() {
        panic!("Finished with errors");
    }
}

fn report_invalid_includes(errors: &[Error]) {
    eprintln!("Invalid includes detected!");
    for error in errors {
        eprintln!("{error}")
    }
}

fn validate_includes(includes: Vec<Include>, valid_anchors: Vec<Anchor>) -> Vec<Error> {
    let (pairs, errors): (Vec<_>, Vec<_>) = includes
        .into_iter()
        .map(|include| {
            let mut maybe_anchor = valid_anchors.iter().find(|anchor| {
                anchor.file == include.anchor_file && anchor.name == include.anchor_name
            });

            match maybe_anchor.take() {
                Some(anchor) => Ok((include, anchor.clone())),
                None => Err(anyhow!(
                    "No anchor available to satisfy include {include:?}"
                )),
            }
        })
        .partition_result();

    let additional_errors = valid_anchors
        .iter()
        .filter(|valid_anchor| {
            let anchor_used_in_a_pair = pairs.iter().any(|(_, anchor)| anchor == *valid_anchor);
            !anchor_used_in_a_pair
        })
        .map(|unused_anchor| anyhow!("anchor unused: {unused_anchor:?}!"))
        .collect::<Vec<_>>();

    chain!(errors, additional_errors).collect()
}

#[allow(dead_code)]
#[derive(Debug)]
struct Include {
    anchor_name: String,
    anchor_file: PathBuf,
    include_file: PathBuf,
    line_no: usize,
}

fn parse_includes(text_w_includes: String) -> Vec<Include> {
    let apply_regex = |regex: Regex| {
        text_w_includes
            .lines()
            .filter_map(|line| regex.captures(line))
            .map(|capture| {
                let include_file = PathBuf::from(&capture[1]).canonicalize().unwrap();
                let line_no = capture[2].parse().unwrap();
                let anchor_file = PathBuf::from(&capture[3]);
                let anchor_name = capture[4].to_owned();

                let the_path = include_file.parent().unwrap().join(anchor_file);

                let anchor_file = the_path.canonicalize();
                if anchor_file.is_err() {
                    panic!(
                        "{the_path:?} when canonicalized gives error {:?}",
                        anchor_file.err().unwrap()
                    )
                }

                let anchor_file = anchor_file.unwrap();

                Include {
                    anchor_name,
                    anchor_file,
                    include_file,
                    line_no,
                }
            })
            .collect::<Vec<_>>()
    };

    apply_regex(Regex::new(r"^(\S+):(\d+):\s*\{\{\s*#include\s*(\S+)\s*:\s*(\S+)\s*\}\}").unwrap())
}

fn report_invalid_anchors(errors: &[Error]) {
    eprintln!("Invalid anchors encountered!");
    for error in errors {
        eprintln!("{error}");
    }
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
            Some(_) => {
                let err_msg = check_validity_of_anchor_pair(&begin, &end).iter().map(|e|e.to_string()).collect::<Vec<_>>().join("\n");
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

fn extract_starts_and_ends(text_w_anchors: &str) -> (Vec<Anchor>, Vec<Anchor>) {
    let apply_regex = |regex: Regex| {
        text_w_anchors
            .lines()
            .filter_map(|line| regex.captures(line))
            .map(|capture| {
                let file = PathBuf::from(&capture[1]).canonicalize().unwrap();
                let line_no = &capture[2];
                let anchor_name = &capture[3];

                Anchor {
                    line_no: line_no.parse().unwrap(),
                    name: anchor_name.to_string(),
                    file,
                }
            })
            .collect::<Vec<_>>()
    };

    let begins = apply_regex(Regex::new(r"^(.+):(\d+):\s*//\s*ANCHOR\s*:\s*(\S+)").unwrap());
    let ends = apply_regex(Regex::new(r"^(.+):(\d+):\s*//\s*ANCHOR_END\s*:\s*(\S+)").unwrap());

    (begins, ends)
}

fn search_for_patterns_in_project(pattern: &str) -> anyhow::Result<String> {
    let grep_project = std::process::Command::new("grep")
        .args([
            "-I",
            "-H",
            "-R",
            "-n",
            "--exclude-dir=scripts",
            pattern,
            ".",
        ])
        .output()
        .expect("failed grep command");

    if !grep_project.status.success() {
        bail!(format!(
            "Failed running grep command for searching {}",
            pattern
        ));
    }

    Ok(String::from_utf8(grep_project.stdout)?)
}
