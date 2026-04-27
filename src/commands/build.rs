use anyhow::Result;

use crate::config::Config;
use crate::dev::harness::Harness;
use crate::dev::opencode::OpenCodeHarness;
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::nix_daemon;
use crate::user::get_user;

pub fn handle_build(cfg: &Config, base: bool, force: bool, no_cache: bool) -> Result<()> {
    let docker = DockerClient;
    let user = get_user()?;
    let opts = BuildOptions { force, no_cache };

    if base {
        nix_daemon::build(&docker, opts)?;
    }

    OpenCodeHarness.ensure_image(&docker, cfg, &user, opts)?;

    Ok(())
}
