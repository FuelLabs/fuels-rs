use octocrab::Octocrab;
use dotenv::dotenv;

pub async fn get_latest_release_tag() -> Result<String, Box<dyn std::error::Error>> {
    dotenv().ok();

    let github_token = std::env::var("GITHUB_TOKEN").ok();

    if let Some(token) = github_token {
        let octocrab = Octocrab::builder().personal_token(token).build()?;

        let repo_owner =
            std::env::var("GITHUB_REPOSITORY_OWNER").expect("Repository owner not found");
        let repo_name = std::env::var("GITHUB_REPOSITORY_NAME").expect("Repository name not found");

        let latest_release = octocrab
            .repos(&repo_owner, &repo_name)
            .releases()
            .get_latest()
            .await?;

        Ok(latest_release.tag_name)
    } else {
        eprintln!("Please add GITHUB_TOKEN to the environment");
        std::process::exit(1);
    }
}
