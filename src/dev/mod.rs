pub mod build;
pub mod container_name;
pub mod env_passthrough;
pub mod extra_dirs;
pub mod image;
pub mod port;
pub mod run;
pub mod shadow_mounts;
pub mod utils;
pub mod volumes;
pub mod workspace;

pub use build::{build_dev, ensure_dev_image};

use crate::config::Config;
use crate::version::github::GithubVersionFetcher;
use crate::version::{get_cache_path, resolve_version};
use anyhow::Result;

/// Resolve the concrete opencode version based on config.
pub fn resolve_opencode_version(config: &Config) -> Result<String> {
    let cache_path = get_cache_path();
    resolve_version(
        &config.opencode_version,
        config.version_cache_ttl_hours,
        &cache_path,
        &GithubVersionFetcher,
    )
}
