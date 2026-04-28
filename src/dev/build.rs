use anyhow::Result;

use crate::config::Config;
use crate::dev::agent::Agent;
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::nix_daemon;
use crate::user::get_user;

/// Build an agent's Docker image and optionally the Nix daemon base image.
pub fn build_agent(
    agent: &dyn Agent,
    cfg: &Config,
    base: bool,
    force: bool,
    no_cache: bool,
) -> Result<()> {
    let docker = DockerClient;
    let user = get_user()?;
    let opts = BuildOptions { force, no_cache };

    if base {
        nix_daemon::build(&docker, opts)?;
    }

    agent.ensure_image(&docker, cfg, &user, opts)?;

    Ok(())
}
