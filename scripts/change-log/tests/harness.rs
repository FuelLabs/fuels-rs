use change_log::ports::github::GitHubPort;
use dotenv::dotenv;
use octocrab::models::repos::Release;
use octocrab::Octocrab;
use std::env;
use tokio;

use change_log::adapters::octocrab::OctocrabAdapter;
use change_log::domain::changelog::generate_changelog;

#[tokio::test]
async fn test_changelog_regression_between_releases() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables (e.g. GITHUB_TOKEN)
    dotenv().ok();

    // Read the GitHub token from the environment.
    let github_token = env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN must be set for e2e tests");

    // Hardcode the repository owner and name.
    let repo_owner = "FuelLabs";
    let repo_name = "fuels-rs";

    // Create an Octocrab client directly.
    let octocrab = Octocrab::builder()
        .personal_token(github_token.clone())
        .build()?;

    // Fetch the list of releases for the repository.
    let releases_page = octocrab
        .repos(repo_owner, repo_name)
        .releases()
        .list()
        .per_page(5)
        .send()
        .await?;
    let mut releases: Vec<Release> = releases_page.items;

    // Ensure that we have at least two releases.
    if releases.len() < 2 {
        eprintln!("Not enough releases to run regression tests (need at least 2).");
        return Ok(());
    }

    // Sort releases by creation date descending (most recent first)
    releases.sort_by(|a, b| b.created_at.partial_cmp(&a.created_at).unwrap());
    let current_release = &releases[0];
    let previous_release = &releases[1];

    // Ensure that both releases have a tag name.
    let current_tag = current_release.tag_name.to_string();
    let previous_tag = previous_release.tag_name.to_string();

    println!(
        "Generating changelog from '{}' to '{}'",
        previous_tag, current_tag
    );

    // Create our GitHub adapter using our refactored code.
    let github_adapter = OctocrabAdapter::new(&github_token);

    // Retrieve changelog infos for commits between the previous and current release.
    // Note: We use the previous release tag as base and the current release tag as head.
    let changelog_infos = github_adapter
        .get_changelog_infos(repo_owner, repo_name, &previous_tag, &current_tag)
        .await?;

    // Generate the markdown changelog.
    let generated_markdown = generate_changelog(changelog_infos);

    // Retrieve the stored release notes (body) from the current release.
    let stored_release_body = current_release
        .body
        .as_ref()
        .map(|s| s.trim())
        .unwrap_or("");

    // Normalize both outputs for a fair comparison. You may want to adjust normalization
    // (for example, by removing extra newlines or whitespace) depending on your formatting.
    let normalized_generated = generated_markdown.trim();
    let normalized_stored = stored_release_body;

    // Print both outputs for debugging (optional).
    println!("--- Generated Markdown ---\n{}\n", normalized_generated);
    println!("--- Stored Release Notes ---\n{}\n", normalized_stored);

    // Compare the generated changelog with the stored release notes.
    // If they do not match exactly, the test will fail, alerting you to a regression.
    pretty_assertions::assert_eq!(
        normalized_generated,
        normalized_stored,
        "The generated changelog does not match the stored release notes for release {}.",
        current_tag
    );

    Ok(())
}
