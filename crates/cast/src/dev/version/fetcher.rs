use anyhow::{Context, Result};

/// Trait for fetching the latest version of an agent.
pub trait VersionFetcher {
    fn fetch_latest_version(&self) -> Result<String>;
}

/// Fetches the latest version from GitHub releases.
pub struct GithubReleaseFetcher {
    pub repo: &'static str,
}

impl VersionFetcher for GithubReleaseFetcher {
    fn fetch_latest_version(&self) -> Result<String> {
        #[derive(serde::Deserialize)]
        struct GithubRelease {
            tag_name: String,
        }

        let url = format!("https://api.github.com/repos/{}/releases/latest", self.repo);
        let release: GithubRelease = ureq::get(&url)
            .set("User-Agent", "cast")
            .call()
            .context("Failed to reach GitHub API")?
            .into_json()
            .context("Failed to parse GitHub API response")?;
        Ok(release.tag_name)
    }
}

/// Fetches the latest version of an npm package from the npm registry.
pub struct NpmRegistryFetcher {
    pub package: &'static str,
}

impl VersionFetcher for NpmRegistryFetcher {
    fn fetch_latest_version(&self) -> Result<String> {
        #[derive(serde::Deserialize)]
        struct NpmLatest {
            version: String,
        }

        let url = format!("https://registry.npmjs.org/{}/latest", self.package);
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(10))
            .build();
        let dist: NpmLatest = agent
            .get(&url)
            .set("User-Agent", "cast")
            .call()
            .context("Failed to reach npm registry")?
            .into_json()
            .context("Failed to parse npm registry response")?;
        Ok(dist.version)
    }
}
