use anyhow::Error;
use check_docs::{
    Anchor, Include, extract_starts_and_ends, filter_valid_anchors, find_files, parse_includes,
    parse_md_files, search_for_pattern, validate_includes, validate_md_files,
};

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
    let test_data = search_for_pattern("ANCHOR", ".")?;

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
    assert!(contains_any(
        &anchor_err_vec,
        "Couldn't find a matching end anchor for Anchor { line_no: 12, name: \"test_no_anchor_end\""
    ));
    assert!(contains_any(
        &anchor_err_vec,
        "The end of the anchor appears before the beginning. End anchor: Anchor { line_no: 14, name: \"test_end_before_beginning\""
    ));

    assert!(contains_any(
        &anchor_err_vec,
        "Found too many matching anchor ends for anchor: Anchor { line_no: 17, name: \"test_same_name_multiple_time\""
    ));
    assert!(contains_any(
        &anchor_err_vec,
        "Found too many matching anchor ends for anchor: Anchor { line_no: 20, name: \"test_same_name_multiple_time\""
    ));
    // Caused by too many matching anchors
    assert!(contains_any(
        &anchor_err_vec,
        "Missing anchor start for Anchor { line_no: 18, name: \"test_same_name_multiple_time\""
    ));
    assert!(contains_any(
        &anchor_err_vec,
        "Missing anchor start for Anchor { line_no: 21, name: \"test_same_name_multiple_time\""
    ));

    let text_mentioning_include = search_for_pattern("{{#include", ".")?;

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
    let text_with_md_files = search_for_pattern(".md", "./tests/test_data/docs/src/SUMMARY.md")?;
    let md_files_in_summary = parse_md_files(text_with_md_files, "./tests/test_data/docs/src/");
    let md_files_in_src = find_files("*.md", "./tests/test_data/docs/src/", "SUMMARY.md")?;
    let md_files_errors = validate_md_files(md_files_in_summary, md_files_in_src);

    let error_msg = md_files_errors.first().unwrap().to_string();

    assert!(error_msg.contains("test-not-there.md` not in SUMMARY.md"));

    Ok(())
}
