use anyhow::{bail, Error};
use check_docs::{
    extract_starts_and_ends, filter_valid_anchors, parse_includes, report_errors,
    search_for_patterns_in_project, validate_includes,
};

fn main() -> anyhow::Result<(), Error> {
    let text_w_anchors = search_for_patterns_in_project("ANCHOR")?;

    let (starts, ends) = extract_starts_and_ends(&text_w_anchors)?;

    let (valid_anchors, anchor_errors) = filter_valid_anchors(starts, ends);

    let text_mentioning_include = search_for_patterns_in_project("{{#include")?;
    let (includes, include_path_errors) = parse_includes(text_mentioning_include);

    let (include_errors, additional_warnings) = validate_includes(includes, valid_anchors);

    report_errors("warning", &additional_warnings);
    report_errors("include paths", &include_path_errors);
    report_errors("anchors", &anchor_errors);
    report_errors("includes", &include_errors);

    if !anchor_errors.is_empty()
        || !include_errors.is_empty()
        || !include_path_errors.is_empty()
        || !additional_warnings.is_empty()
    {
        bail!("Finished with errors");
    }

    Ok(())
}
