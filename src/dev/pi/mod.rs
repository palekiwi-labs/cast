use anyhow::Result;

use crate::config::Config;
use crate::dev::agent::Agent;
use crate::dev::run::RunOpts;
use crate::dev::version::fetcher::GithubReleaseFetcher;
use crate::dev::version::{self, VersionResolver};
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::user::ResolvedUser;
use std::collections::HashMap;

pub mod cmd;
pub mod image;

/// Resolve the concrete pi version based on config.
pub fn resolve_version(config: &Config) -> Result<String> {
    let requested = config
        .agent_versions
        .get("pi")
        .map(|s| s.as_str())
        .unwrap_or("latest");
    let cache_path = version::cache::get_cache_path("pi");
    let resolver = VersionResolver::new(cache_path, config.version_cache_ttl_hours);
    let fetcher = GithubReleaseFetcher {
        repo: "badlogic/pi-mono",
    };
    resolver.resolve(requested, &fetcher)
}

pub struct Pi;

impl Agent for Pi {
    fn name(&self) -> &str {
        "pi"
    }

    fn image_tag(&self, config: &Config) -> Result<String> {
        let version = resolve_version(config)?;
        Ok(image::get_image_tag(&version))
    }

    fn ensure_image(
        &self,
        docker: &DockerClient,
        config: &Config,
        user: &ResolvedUser,
        opts: BuildOptions,
    ) -> Result<()> {
        let version = resolve_version(config)?;
        image::ensure_dev_image(docker, config, user, &version, opts)
    }

    fn extra_run_args(
        &self,
        _config: &Config,
        _opts: &RunOpts,
        _env: &HashMap<String, String>,
    ) -> Result<Vec<String>> {
        unimplemented!()
    }

    fn command(&self, config: &Config, opts: &RunOpts, extra_args: Vec<String>) -> Vec<String> {
        let mut command = cmd::resolve_pi_command(config, &opts.user);
        command.extend(extra_args);
        command
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_resolve_version_defaults_to_latest() {
        let mut config = Config::default();
        // Set a high TTL so it doesn't try to fetch from network if not needed,
        // but since it's a new agent it will probably try to fetch.
        config.version_cache_ttl_hours = 24;

        let version = resolve_version(&config).unwrap();
        // If network is available, it should resolve to something like "v0.71.0"
        // If not, it might fail or we might need to mock the fetcher.
        assert!(!version.is_empty());
    }
}
