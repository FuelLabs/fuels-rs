use crate::lib::{
    extract_starts_and_ends, filter_valid_anchors, parse_includes, report_errors, report_warnings,
    search_for_patterns_in_project, validate_includes,
};
use anyhow::{bail, Error};
pub mod lib;

fn main() -> anyhow::Result<(), Error> {
    let text_w_anchors = search_for_patterns_in_project("ANCHOR")?;

    let (starts, ends) = extract_starts_and_ends(&text_w_anchors)?;

    let (valid_anchors, anchor_errors) = filter_valid_anchors(starts, ends);

    let text_mentioning_include = search_for_patterns_in_project("{{#include")?;
    let (includes, include_path_errors) = parse_includes(text_mentioning_include);

    let (include_errors, additional_warnings) = validate_includes(includes, valid_anchors);

    if !additional_warnings.is_empty() {
        report_warnings(&additional_warnings);
    }

    if !anchor_errors.is_empty() || !include_errors.is_empty() || !include_path_errors.is_empty() {
        report_errors("include paths", &include_path_errors);
        report_errors("anchors", &anchor_errors);
        report_errors("includes", &include_errors);
        bail!("Finished with errors");
    }
    Ok(())
}
