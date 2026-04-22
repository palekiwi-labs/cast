use anyhow::Result;

use crate::config::Config;
use crate::dev;
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::nix;
use crate::opencode;
use crate::user::get_user;

pub fn handle_build(cfg: &Config, base: bool, force: bool, no_cache: bool) -> Result<()> {
    let docker = DockerClient;
    let user = get_user()?;

    let version = opencode::resolve_version(cfg)?;

    let opts = BuildOptions { force, no_cache };

    if base {
        nix::build(&docker, opts)?;
    }

    dev::build_dev(&docker, cfg, &user, &version, opts)?;

    Ok(())
}
