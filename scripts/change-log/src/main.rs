mod get_full_changelog;
mod get_latest_release;

use get_full_changelog::{generate_changelog, get_changelogs, write_changelog_to_file};
use get_latest_release::get_latest_release_tag;
use octocrab::Octocrab;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let github_token =
        std::env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN is not set in the environment");
    let repo_owner = std::env::var("GITHUB_REPOSITORY_OWNER").expect("Repository owner not found");
    let repo_name = std::env::var("GITHUB_REPOSITORY_NAME").expect("Repository name not found");

    let octocrab = Octocrab::builder().personal_token(github_token).build()?;

    let latest_release_tag = get_latest_release_tag().await?;

    let changelogs = get_changelogs(
        &octocrab,
        &repo_owner,
        &repo_name,
        &latest_release_tag,
        "master",
    )
    .await?;

    let full_changelog = generate_changelog(changelogs);

    write_changelog_to_file(&full_changelog, "output_changelog.md")?;

    Ok(())
}
