use std::collections::{HashMap, HashSet};

use crate::domain::models::ChangelogInfo;

fn category_from_pr_type(pr_type: &str) -> Option<&'static str> {
    match pr_type.trim_end_matches('!') {
        "feat" => Some("Features"),
        "fix" => Some("Fixes"),
        "chore" => Some("Chores"),
        _ => None,
    }
}

pub fn generate_changelog(changelogs: Vec<ChangelogInfo>) -> String {
    let mut content = String::new();

    let mut non_breaking: HashMap<&str, Vec<String>> = HashMap::new();
    let mut breaking: HashMap<&str, Vec<String>> = HashMap::new();
    let mut migration_notes: Vec<String> = Vec::new();
    let mut summary_set: HashSet<String> = HashSet::new();

    for changelog in &changelogs {
        if !changelog.release_notes.is_empty() {
            summary_set.insert(changelog.release_notes.clone());
        }
        if let Some(category) = category_from_pr_type(&changelog.pr_type) {
            if changelog.is_breaking {
                breaking
                    .entry(category)
                    .or_default()
                    .push(changelog.bullet_point.clone());
                migration_notes.push(changelog.migration_note.clone());
            } else {
                non_breaking
                    .entry(category)
                    .or_default()
                    .push(changelog.bullet_point.clone());
            }
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

    let categories = ["Features", "Fixes", "Chores"];
    if !breaking.is_empty() {
        content.push_str("# Breaking\n\n");
        for cat in &categories {
            if let Some(items) = breaking.get(cat) {
                content.push_str(&format!("- {}\n", cat));

                let indented = items
                    .iter()
                    .map(|s| format!("\t{}", s))
                    .collect::<Vec<_>>()
                    .join("\n");
                content.push_str(&format!("{}\n\n", indented));
            }
        }
    }

    let mut write_section = |title: &str, items: &[String]| {
        if !items.is_empty() {
            content.push_str(&format!("# {}\n\n", title));
            content.push_str(&format!("{}\n\n", items.join("\n\n")));
        }
    };

    for cat in &categories {
        if let Some(items) = non_breaking.get(cat) {
            write_section(cat, items);
        }
    }

    if !migration_notes.is_empty() {
        write_section("Migration Notes", &migration_notes);
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
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::ChangelogInfo;

    #[test]
    fn test_generate_changelog_exact() {
        let changelog1 = ChangelogInfo {
            is_breaking: false,
            pr_type: "feat".to_string(),
            bullet_point: "- [#1](http://example.com) - Added feature, by @alice".to_string(),
            migration_note: "".to_string(),
            release_notes: "Added feature".to_string(),
        };

        let changelog2 = ChangelogInfo {
            is_breaking: true,
            pr_type: "fix!".to_string(),
            bullet_point: "- [#2](http://example.com) - Fixed bug, by @bob".to_string(),
            migration_note: "### [2 - Fixed bug](http://example.com)\n\nCritical fix".to_string(),
            release_notes: "Fixed bug".to_string(),
        };

        let changelog3 = ChangelogInfo {
            is_breaking: false,
            pr_type: "chore".to_string(),
            bullet_point: "- [#3](http://example.com) - Update dependencies, by @carol".to_string(),
            migration_note: "".to_string(),
            release_notes: "".to_string(),
        };

        let changelogs = vec![changelog1, changelog2, changelog3];
        let markdown = generate_changelog(changelogs);

        let expected = "\
# Summary

In this release, we:
Added feature
Fixed bug

# Breaking

- Fixes
\t- [#2](http://example.com) - Fixed bug, by @bob

# Features

- [#1](http://example.com) - Added feature, by @alice

# Chores

- [#3](http://example.com) - Update dependencies, by @carol

# Migration Notes

### [2 - Fixed bug](http://example.com)

Critical fix";

        assert_eq!(markdown, expected);
    }
}
