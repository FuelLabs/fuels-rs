use anyhow::{Error, bail};
use check_docs::{
    extract_starts_and_ends, filter_valid_anchors, find_files, parse_includes, parse_md_files,
    report_errors, search_for_pattern, validate_includes, validate_md_files,
};

fn main() -> anyhow::Result<(), Error> {
    let text_w_anchors = search_for_pattern("ANCHOR", ".")?;
    let (starts, ends) = extract_starts_and_ends(&text_w_anchors)?;
    let (valid_anchors, anchor_errors) = filter_valid_anchors(starts, ends);

    let text_mentioning_include = search_for_pattern("{{#include", ".")?;
    let (includes, include_path_errors) = parse_includes(text_mentioning_include);
    let (include_errors, additional_warnings) = validate_includes(includes, valid_anchors);

    let text_with_md_files = search_for_pattern(".md", "./docs/src/SUMMARY.md")?;
    let md_files_in_summary = parse_md_files(text_with_md_files, "./docs/src/");
    let md_files_in_src = find_files("*.md", "./docs/src/", "SUMMARY.md")?;
    let md_files_errors = validate_md_files(md_files_in_summary, md_files_in_src);

    report_errors("warning", &additional_warnings);
    report_errors("include paths", &include_path_errors);
    report_errors("anchors", &anchor_errors);
    report_errors("includes", &include_errors);
    report_errors("md files", &md_files_errors);

    if !anchor_errors.is_empty()
        || !include_errors.is_empty()
        || !include_path_errors.is_empty()
        || !additional_warnings.is_empty()
        || !md_files_errors.is_empty()
    {
        bail!("Finished with errors");
    }

    Ok(())
}
