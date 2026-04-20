use anyhow::Result;

use crate::config::Config;
use crate::dev;
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::nix;
use crate::user::get_user;
use crate::version::github::GithubVersionFetcher;
use crate::version::{get_cache_path, resolve_version};

pub fn handle_build(cfg: &Config, base: bool, force: bool, no_cache: bool) -> Result<()> {
    let docker = DockerClient;
    let user = get_user()?;
    let cache_dir = get_cache_path();

    let version = resolve_version(
        &cfg.opencode_version,
        cfg.version_cache_ttl_hours,
        &cache_dir,
        &GithubVersionFetcher,
    )?;

    let opts = BuildOptions { force, no_cache };

    if base {
        nix::build(&docker, opts)?;
    }

    dev::build_dev(&docker, cfg, &user, &version, opts)?;

    Ok(())
}
