use crate::domain::models::ChangelogInfo;

#[allow(async_fn_in_trait)]
pub trait GitHubPort {
    /// Retrieve a collection of changelog infos based on the commit comparison between `base` and `head`.
    async fn get_changelog_infos(
        &self,
        owner: &str,
        repo: &str,
        base: &str,
        head: &str,
    ) -> Result<Vec<ChangelogInfo>, Box<dyn std::error::Error>>;

    /// Retrieve the latest release tag for the given repository.
    async fn get_latest_release_tag(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<String, Box<dyn std::error::Error>>;
}
