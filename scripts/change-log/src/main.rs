use std::env;

use change_log::{
    adapters::octocrab::OctocrabAdapter, domain::changelog::generate_changelog,
    ports::github::GitHubPort,
};
use dialoguer::FuzzySelect;
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let github_token =
        env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN is not set in the environment");
    let repo_owner = env::var("GITHUB_REPOSITORY_OWNER").unwrap_or_else(|_| "FuelLabs".to_string());
    let repo_name = env::var("GITHUB_REPOSITORY_NAME").unwrap_or_else(|_| "fuels-rs".to_string());

    let github_adapter = OctocrabAdapter::new(&github_token);

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

    eprintln!("Using branch: {}", target_branch);
    eprintln!("Using previous release: {}", previous_release_tag);

    let changelog_infos = github_adapter
        .get_changelog_infos(
            &repo_owner,
            &repo_name,
            &previous_release_tag,
            &target_branch,
        )
        .await?;

    let changelog_markdown = generate_changelog(changelog_infos);

    println!("{changelog_markdown}");

    Ok(())
}
