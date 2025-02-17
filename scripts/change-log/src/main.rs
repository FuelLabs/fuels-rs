use change_log::adapters::file_changelog_writer::FileChangelogWriter;
use change_log::adapters::octocrab::OctocrabAdapter;
use change_log::domain::changelog::generate_changelog;
use change_log::ports::changelog_writer::ChangelogWriter;
use change_log::ports::github::GitHubPort;
use dialoguer::FuzzySelect;
use dotenv::dotenv;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    // Read configuration from environment variables.
    let github_token =
        env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN is not set in the environment");
    let repo_owner = env::var("GITHUB_REPOSITORY_OWNER").unwrap_or_else(|_| "FuelLabs".to_string());
    let repo_name = env::var("GITHUB_REPOSITORY_NAME").unwrap_or_else(|_| "fuels-rs".to_string());

    // Create our GitHub adapter.
    let github_adapter = OctocrabAdapter::new(&github_token);

    // Query GitHub for available branches.

    let branches = {
        let mut branches = vec!["master".to_string()];
        let lts_branches = github_adapter
            .search_branches(&repo_owner, &repo_name, "lts/")
            .await?;
        branches.extend(lts_branches);
        branches
    };

    let branch_selection = FuzzySelect::new()
        .with_prompt("Select the target branch (start typing to filter)")
        .items(&branches)
        .default(0)
        .interact()?;

    let target_branch = branches[branch_selection].clone();

    // Query GitHub for available releases.
    let releases = github_adapter.get_releases(&repo_owner, &repo_name).await?;
    if releases.is_empty() {
        return Err("No releases found for the repository".into());
    }
    let release_selection = FuzzySelect::new()
        .with_prompt("Select the previous release tag")
        .items(&releases)
        .default(0)
        .interact()?;
    let previous_release_tag = releases[release_selection].clone();

    println!("Using branch: {}", target_branch);
    println!("Using previous release: {}", previous_release_tag);

    // Generate changelog infos by comparing the previous release tag with the target branch.
    let changelog_infos = github_adapter
        .get_changelog_infos(
            &repo_owner,
            &repo_name,
            &previous_release_tag,
            &target_branch,
        )
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
