pub mod version;

use crate::config::Config;
use crate::opencode::version::github::GithubVersionFetcher;
use crate::opencode::version::{get_cache_path, resolve_version as do_resolve_version};
use anyhow::Result;

/// Resolve the concrete opencode version based on config.
pub fn resolve_version(config: &Config) -> Result<String> {
    let cache_path = get_cache_path();
    do_resolve_version(
        &config.opencode_version,
        config.version_cache_ttl_hours,
        &cache_path,
        &GithubVersionFetcher,
    )
}
