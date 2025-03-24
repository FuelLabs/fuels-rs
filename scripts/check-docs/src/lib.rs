use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use anyhow::{Error, anyhow, bail};
use itertools::{Itertools, chain};
use regex::Regex;

pub fn report_errors(error_type: &str, errors: &[Error]) {
    if !errors.is_empty() {
        eprintln!("\nInvalid {error_type} detected!\n");
        for error in errors {
            eprintln!("{error}\n")
        }
    }
}

pub fn report_warnings(warnings: &[Error]) {
    if !warnings.is_empty() {
        eprintln!("\nWarnings detected!\n");
        for warning in warnings {
            eprintln!("{warning}\n")
        }
    }
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
        Some(anyhow!(
            "The end of the anchor appears before the beginning. End anchor: {end:?}. Begin anchor: {begin:?}"
        ))
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

pub fn parse_md_files(text_w_files: String, path: &str) -> HashSet<PathBuf> {
    let regex = Regex::new(r"\((.*\.md)\)").expect("could not construct regex");

    text_w_files
        .lines()
        .filter_map(|line| regex.captures(line))
        .map(|capture| {
            let path = PathBuf::from(path).join(&capture[1]);
            path.canonicalize()
                .unwrap_or_else(|e| panic!("could not canonicalize md path: {e} {path:?}"))
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

pub fn search_for_pattern(pattern: &str, location: &str) -> anyhow::Result<String> {
    let grep_project = std::process::Command::new("grep")
        .arg("-H") // print filename
        .arg("-n") // print line-number
        .arg("-r") // search recursively
        .arg("--binary-files=without-match")
        .arg("--exclude-dir=check-docs")
        .arg(pattern)
        .arg(location)
        .output()
        .expect("failed grep command");

    if !grep_project.status.success() {
        bail!("Failed running `grep` command for pattern '{}'", pattern);
    }

    Ok(String::from_utf8(grep_project.stdout)?)
}

pub fn find_files(pattern: &str, location: &str, exclude: &str) -> anyhow::Result<String> {
    let find = std::process::Command::new("find")
        .args([
            location, "-type", "f", "-name", pattern, "!", "-name", exclude,
        ])
        .output()
        .expect("Program `find` not in PATH");

    if !find.status.success() {
        bail!("Failed running `find` command for pattern {}", pattern);
    }

    Ok(String::from_utf8(find.stdout)?)
}
