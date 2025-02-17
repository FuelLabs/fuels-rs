// File: src/main.rs

mod adapters;
mod domain;
mod ports;

use adapters::file_changelog_writer::FileChangelogWriter;
use adapters::octocrab::OctocrabAdapter;
use domain::changelog::generate_changelog;
use dotenv::dotenv;
use ports::changelog_writer::ChangelogWriter;
use ports::github::GitHubPort;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    // Read configuration from environment variables.
    let github_token =
        env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN is not set in the environment");
    let repo_owner =
        env::var("GITHUB_REPOSITORY_OWNER").expect("GITHUB_REPOSITORY_OWNER is not set");
    let repo_name = env::var("GITHUB_REPOSITORY_NAME").expect("GITHUB_REPOSITORY_NAME is not set");

    // Create our GitHub adapter.
    let github_adapter = OctocrabAdapter::new(&github_token);

    // Retrieve the latest release tag.
    let latest_release_tag = github_adapter
        .get_latest_release_tag(&repo_owner, &repo_name)
        .await?;

    // Define the branch we’re comparing against.
    let head_branch = "master";

    // Get changelog infos from GitHub.
    let changelog_infos = github_adapter
        .get_changelog_infos(&repo_owner, &repo_name, &latest_release_tag, head_branch)
        .await?;

    // Generate the markdown changelog.
    let changelog_markdown = generate_changelog(changelog_infos);

    // Create our file writer adapter.
    let writer = FileChangelogWriter {
        file_path: "output_changelog.md".to_string(),
    };

    // Write the changelog to the file.
    writer.write_changelog(&changelog_markdown)?;

    println!("Changelog written to output_changelog.md");

    Ok(())
}
