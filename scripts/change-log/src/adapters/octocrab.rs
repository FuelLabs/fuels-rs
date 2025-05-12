use octocrab::{Octocrab, models::pulls::PullRequest};
use regex::Regex;
use serde_json::Value;

use crate::{
    domain::{changelog::capitalize, models::ChangelogInfo},
    ports::github::GitHubPort,
};

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

    pub async fn search_branches(
        &self,
        owner: &str,
        repo: &str,
        query: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let payload = serde_json::json!({
            "query": r#"
                query($owner: String!, $repo: String!, $query: String!) {
                    repository(owner: $owner, name: $repo) {
                        refs(refPrefix: "refs/heads/", query: $query, first: 100) {
                            nodes {
                                name
                            }
                        }
                    }
                }
            "#,
            "variables": {
                "owner": owner,
                "repo": repo,
                "query": query,
            }
        });

        let response: Value = self.client.graphql(&payload).await?;

        let nodes = response["data"]["repository"]["refs"]["nodes"]
            .as_array()
            .ok_or("Could not parse branch nodes from response")?;

        let branch_names = nodes
            .iter()
            .filter_map(|node| node["name"].as_str().map(|s| s.to_owned()))
            .collect();

        Ok(branch_names)
    }

    /// Query GitHub for all releases in the repository.
    pub async fn get_releases(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let releases = self
            .client
            .repos(owner, repo)
            .releases()
            .list()
            .per_page(100)
            .send()
            .await?;

        let release_tags = releases
            .items
            .into_iter()
            .map(|release| release.tag_name)
            .collect();

        Ok(release_tags)
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

        for commit in comparison.commits {
            match self.build_changelog_info(owner, repo, &commit.sha).await {
                Ok(info) => changelogs.push(info),
                Err(e) => {
                    eprintln!("Error retrieving PR for commit {}: {}", commit.sha, e);
                    continue;
                }
            }
        }

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
