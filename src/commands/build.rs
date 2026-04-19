use anyhow::Result;
use std::path::PathBuf;

use crate::config::Config;
use crate::nix;
use crate::nix::DockerCliClient;
use crate::user::get_user;
use crate::version::github::GithubVersionFetcher;
use crate::version::resolve_version;

pub fn handle_build(cfg: &Config, base: bool, force: bool, no_cache: bool) -> Result<()> {
    let docker = DockerCliClient;
    let user = get_user()?;
    let fetcher = GithubVersionFetcher;

    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from(".cache"))
        .join("ocx")
        .join("version-cache.json");

    let version = resolve_version(
        &cfg.opencode_version,
        cfg.version_cache_ttl_hours,
        &cache_dir,
        &fetcher,
    )?;

    if base {
        nix::build(&docker)?;
    }

    nix::build_dev(&docker, cfg, &user, &version, force, no_cache)?;

    Ok(())
}
