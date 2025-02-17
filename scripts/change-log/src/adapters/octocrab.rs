use crate::domain::changelog::capitalize;
use crate::domain::models::ChangelogInfo;
use crate::ports::github::GitHubPort;
use octocrab::models::pulls::PullRequest;
use octocrab::Octocrab;
use regex::Regex;
use url::Url;

pub struct OctocrabAdapter {
    client: Octocrab,
}

impl OctocrabAdapter {
    pub fn new(token: &str) -> Self {
        let client = Octocrab::builder()
            .personal_token(token.to_string())
            .build()
            .unwrap();
        Self { client }
    }

    /// Retrieve the pull request associated with a commit SHA.
    async fn get_pr_for_commit(
        &self,
        owner: &str,
        repo: &str,
        commit_sha: &str,
    ) -> Result<PullRequest, Box<dyn std::error::Error>> {
        let pr_info = self
            .client
            .repos(owner, repo)
            .list_pulls(commit_sha.to_string())
            .send()
            .await?;

        if pr_info.items.is_empty() {
            return Err("No PR found for this commit SHA".into());
        }

        let pr = pr_info.items.into_iter().next().unwrap();

        // Ignore PRs from "fuel-service-user"
        if pr.user.as_ref().map_or("", |u| &u.login) == "fuel-service-user" {
            return Err("PR from fuel-service-user ignored".into());
        }

        Ok(pr)
    }

    /// Build a ChangelogInfo instance from a commit.
    async fn build_changelog_info(
        &self,
        owner: &str,
        repo: &str,
        commit_sha: &str,
    ) -> Result<ChangelogInfo, Box<dyn std::error::Error>> {
        let pr = self.get_pr_for_commit(owner, repo, commit_sha).await?;

        let pr_title_full = pr.title.as_ref().unwrap_or(&"".to_string()).clone();
        let pr_type = pr_title_full
            .split(':')
            .next()
            .unwrap_or("misc")
            .to_string();
        let is_breaking = pr_title_full.contains('!');
        let title_description = pr_title_full
            .split(':')
            .nth(1)
            .unwrap_or("")
            .trim()
            .to_string();
        let pr_number = pr.number;
        let pr_author = pr.user.as_ref().map_or("", |u| &u.login).to_string();
        let pr_url = pr.html_url.map(|u| u.to_string()).unwrap_or_default();

        let bullet_point = format!(
            "- [#{}]({}) - {}, by @{}",
            pr_number, pr_url, title_description, pr_author
        );

        let breaking_changes_regex = Regex::new(r"(?s)# Breaking Changes\s*(.*)")?;
        let breaking_changes = breaking_changes_regex
            .captures(pr.body.as_ref().unwrap_or(&String::new()))
            .and_then(|cap| cap.get(1))
            .map(|m| {
                m.as_str()
                    .split("\n# ")
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string()
            })
            .unwrap_or_default();

        let release_notes_regex = Regex::new(r"(?s)In this release, we:\s*(.*)")?;
        let release_notes = release_notes_regex
            .captures(pr.body.as_ref().unwrap_or(&String::new()))
            .and_then(|cap| cap.get(1))
            .map(|m| {
                m.as_str()
                    .split("\n# ")
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string()
            })
            .unwrap_or_default();

        let migration_note = format!(
            "### [{} - {}]({})\n\n{}",
            pr_number,
            capitalize(&title_description),
            pr_url,
            breaking_changes
        );

        Ok(ChangelogInfo {
            is_breaking,
            pr_type,
            bullet_point,
            migration_note,
            release_notes,
            pr_number,
            pr_title: title_description,
            pr_author,
            pr_url,
        })
    }
}

impl GitHubPort for OctocrabAdapter {
    async fn get_changelog_infos(
        &self,
        owner: &str,
        repo: &str,
        base: &str,
        head: &str,
    ) -> Result<Vec<ChangelogInfo>, Box<dyn std::error::Error>> {
        let comparison = self
            .client
            .commits(owner, repo)
            .compare(base, head)
            .send()
            .await?;

        let mut changelogs = Vec::new();

        // For each commit in the comparison, try to build changelog info.
        for commit in comparison.commits {
            match self.build_changelog_info(owner, repo, &commit.sha).await {
                Ok(info) => changelogs.push(info),
                Err(e) => {
                    println!("Error retrieving PR for commit {}: {}", commit.sha, e);
                    continue;
                }
            }
        }

        // Sort by PR type (you can adjust the sort criteria as needed)
        changelogs.sort_by(|a, b| a.pr_type.cmp(&b.pr_type));

        Ok(changelogs)
    }

    async fn get_latest_release_tag(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let latest_release = self
            .client
            .repos(owner, repo)
            .releases()
            .get_latest()
            .await?;
        Ok(latest_release.tag_name)
    }
}
