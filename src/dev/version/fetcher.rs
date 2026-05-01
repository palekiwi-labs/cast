use anyhow::{Context, Result};

/// Trait for fetching the latest version of an agent.
pub trait VersionFetcher {
    fn fetch_latest_version(&self) -> Result<String>;
}

/// Fetches the latest version from GitHub releases.
pub struct GithubReleaseFetcher {
    pub repo: String,
}

impl VersionFetcher for GithubReleaseFetcher {
    fn fetch_latest_version(&self) -> Result<String> {
        #[derive(serde::Deserialize)]
        struct GithubRelease {
            tag_name: String,
        }

        let url = format!("https://api.github.com/repos/{}/releases/latest", self.repo);
        let release: GithubRelease = ureq::get(&url)
            .header("User-Agent", "cast")
            .call()
            .context("Failed to reach GitHub API")?
            .body_mut()
            .read_json()
            .context("Failed to parse GitHub API response")?;
        Ok(release.tag_name)
    }
}
