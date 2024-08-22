use octocrab::Octocrab;
use regex::Regex;
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, Write};

#[derive(Debug)]
pub struct ChangelogInfo {
    pub is_breaking: bool,
    pub pr_type: String,
    pub bullet_point: String,
    pub migration_note: String,
    pub release_notes: String,
    pub pr_number: u64,
    pub pr_title: String,
    pub pr_author: String,
    pub pr_url: String,
}

pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

pub async fn get_changelog_info(
    octocrab: &Octocrab,
    owner: &str,
    repo: &str,
    commit_sha: &str,
) -> Result<ChangelogInfo, Box<dyn std::error::Error>> {
    let pr_info = octocrab
        .repos(owner, repo)
        .list_pulls(commit_sha.to_string())
        .send()
        .await?;
    
    if pr_info.items.is_empty() {
        return Err("No PR found for this commit SHA".into());
    }
    
    let pr = &pr_info.items[0];

    // Skip PRs from the user "fuel-service-user"
    if pr.user.as_ref().map_or("", |user| &user.login) == "fuel-service-user" {
        return Err("PR from fuel-service-user ignored".into());
    }

    let pr_type = pr
        .title
        .as_ref()
        .map_or("misc", |title| title.split(':').next().unwrap_or("misc"))
        .to_string();
    let is_breaking = pr.title.as_ref().map_or(false, |title| title.contains("!"));

    let title_description = pr
        .title
        .as_ref()
        .map_or("", |title| title.split(':').nth(1).unwrap_or(""))
        .trim()
        .to_string();
    let pr_number = pr.number;
    let pr_title = title_description.clone();
    let pr_author = pr.user.as_ref().map_or("", |user| &user.login).to_string();
    let pr_url = pr.html_url.as_ref().map_or("", |url| url.as_str()).to_string();

    let bullet_point = format!(
        "- [#{}]({}) - {}, by @{}",
        pr_number, pr_url, pr_title, pr_author
    );

    let breaking_changes_regex = Regex::new(r"(?s)# Breaking Changes\s*(.*)").unwrap();
    let breaking_changes = breaking_changes_regex
        .captures(&pr.body.as_ref().unwrap_or(&String::new()))
        .map_or_else(|| String::new(), |cap| {
            cap.get(1).map_or(String::new(), |m| {
                m.as_str()
                    .split("\n# ")
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string()
            })
        });

    let release_notes_regex = Regex::new(r"(?s)In this release, we:\s*(.*)").unwrap();
    let release_notes = release_notes_regex
        .captures(&pr.body.as_ref().unwrap_or(&String::new()))
        .map_or_else(|| String::new(), |cap| {
            cap.get(1).map_or(String::new(), |m| {
                m.as_str()
                    .split("\n# ")
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string()
            })
        });

    let migration_note = format!(
        "### [{} - {}]({})\n\n{}",
        pr_number, capitalize(&title_description), pr_url, breaking_changes
    );

    Ok(ChangelogInfo {
        is_breaking,
        pr_type,
        bullet_point,
        migration_note,
        release_notes,
        pr_number,
        pr_title,
        pr_author,
        pr_url,
    })
}

pub async fn get_changelogs(
    octocrab: &Octocrab,
    owner: &str,
    repo: &str,
    base: &str,
    head: &str,
) -> Result<Vec<ChangelogInfo>, Box<dyn std::error::Error>> {
    let comparison = octocrab.commits(owner, repo).compare(base, head).send().await?;
    
    let mut changelogs = Vec::new();

    for commit in comparison.commits {
        match get_changelog_info(&octocrab, owner, repo, &commit.sha).await {
            Ok(info) => changelogs.push(info),
            Err(e) => {
                println!("Error retrieving PR for commit {}: {}", commit.sha, e);
                continue;
            }
        }
    }

    changelogs.sort_by(|a, b| a.pr_type.cmp(&b.pr_type));

    Ok(changelogs)
}

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

        if !changelog.release_notes.is_empty() && !summary_set.contains(&changelog.release_notes) {
            summary_set.insert(format!("{}", changelog.release_notes.clone()));
        }
    }
    
    if !summary_set.is_empty() {
        content.push_str("# Summary\n\nIn this release, we:\n");
        let mut summary_lines: Vec<String> = summary_set.into_iter().collect();
        summary_lines.sort();
        for line in summary_lines {
            content.push_str(&format!("{}\n", line));
        }
        content.push_str("\n");
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

pub fn write_changelog_to_file(changelog: &str, file_path: &str) -> io::Result<()> {
    let mut file = File::create(file_path)?;
    file.write_all(changelog.as_bytes())?;
    Ok(())
}
