use crate::domain::models::ChangelogInfo;
use std::collections::HashSet;

/// Given a list of changelog items, generate a markdown changelog.
pub fn generate_changelog(changelogs: Vec<ChangelogInfo>) -> String {
    let mut content = String::new();

    // Categorize PRs by type
    let mut features = Vec::new();
    let mut fixes = Vec::new();
    let mut chores = Vec::new();
    let mut breaking_features = Vec::new();
    let mut breaking_fixes = Vec::new();
    let mut breaking_chores = Vec::new();
    let mut migration_notes = Vec::new();
    let mut summary_set: HashSet<String> = HashSet::new();

    for changelog in &changelogs {
        if changelog.is_breaking {
            match changelog.pr_type.as_str() {
                "feat!" => breaking_features.push(changelog.bullet_point.clone()),
                "fix!" => breaking_fixes.push(changelog.bullet_point.clone()),
                "chore!" => breaking_chores.push(changelog.bullet_point.clone()),
                _ => {}
            }
            migration_notes.push(changelog.migration_note.clone());
        } else {
            match changelog.pr_type.as_str() {
                "feat" => features.push(changelog.bullet_point.clone()),
                "fix" => fixes.push(changelog.bullet_point.clone()),
                "chore" => chores.push(changelog.bullet_point.clone()),
                _ => {}
            }
        }

        if !changelog.release_notes.is_empty() {
            summary_set.insert(changelog.release_notes.clone());
        }
    }

    if !summary_set.is_empty() {
        content.push_str("# Summary\n\nIn this release, we:\n");
        let mut summary_lines: Vec<String> = summary_set.into_iter().collect();
        summary_lines.sort();
        for line in summary_lines {
            content.push_str(&format!("{}\n", line));
        }
        content.push('\n');
    }

    // Generate the breaking changes section
    if !breaking_features.is_empty() || !breaking_fixes.is_empty() || !breaking_chores.is_empty() {
        content.push_str("# Breaking\n\n");
        if !breaking_features.is_empty() {
            content.push_str("- Features\n");
            content.push_str(&format!("\t{}\n\n", breaking_features.join("\n\t")));
        }
        if !breaking_fixes.is_empty() {
            content.push_str("- Fixes\n");
            content.push_str(&format!("\t{}\n\n", breaking_fixes.join("\n\t")));
        }
        if !breaking_chores.is_empty() {
            content.push_str("- Chores\n");
            content.push_str(&format!("\t{}\n\n", breaking_chores.join("\n\t")));
        }
    }

    // Generate the categorized sections for non-breaking changes
    if !features.is_empty() {
        content.push_str("# Features\n\n");
        content.push_str(&format!("{}\n\n", features.join("\n\n")));
    }
    if !fixes.is_empty() {
        content.push_str("# Fixes\n\n");
        content.push_str(&format!("{}\n\n", fixes.join("\n\n")));
    }
    if !chores.is_empty() {
        content.push_str("# Chores\n\n");
        content.push_str(&format!("{}\n\n", chores.join("\n\n")));
    }

    // Generate the migration notes section
    if !migration_notes.is_empty() {
        content.push_str("# Migration Notes\n\n");
        content.push_str(&format!("{}\n\n", migration_notes.join("\n\n")));
    }

    content.trim().to_string()
}

/// Utility function to capitalize a string.
pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
