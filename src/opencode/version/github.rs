use anyhow::{Context, Result};
use serde::Deserialize;

const GITHUB_API_URL: &str = "https://api.github.com/repos/anomalyco/opencode/releases/latest";

/// Trait for fetching the latest OpenCode version.
/// Allows injection of a test double without hitting the real network.
pub trait VersionFetcher {
    fn fetch_latest_version(&self) -> Result<String>;
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
}

/// Real implementation that calls the GitHub releases API.
pub struct GithubVersionFetcher;

impl VersionFetcher for GithubVersionFetcher {
    fn fetch_latest_version(&self) -> Result<String> {
        let release: GithubRelease = ureq::get(GITHUB_API_URL)
            .header("User-Agent", "ocx")
            .call()
            .context("Failed to reach GitHub API")?
            .body_mut()
            .read_json()
            .context("Failed to parse GitHub API response")?;
        Ok(release.tag_name)
    }
}
