use anyhow::Result;

use crate::config::ApprovedConfig;
use crate::dev::agent::Agent;
use crate::dev::image;
use crate::docker::BuildOptions;
use crate::docker::client::DockerClient;
use crate::nix_daemon;
use crate::user::get_user;

/// Build the single shared dev image and optionally the Nix daemon base image.
///
/// The image is harness-free and agent-agnostic; `_agent` is retained so the
/// per-agent `cast build <agent>` CLI surface can dispatch here unchanged.
pub fn build_agent(
    _agent: &dyn Agent,
    cfg: &ApprovedConfig,
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

    image::ensure_dev_image(&docker, cfg, &user, opts)?;

    Ok(())
}
