use anyhow::Result;

use crate::config::ApprovedConfig;
use crate::dev::image;
use crate::docker::BuildOptions;
use crate::docker::client::DockerClient;
use crate::nix_daemon;
use crate::user::get_user;

/// Build the single shared dev image and optionally the Nix daemon image.
///
/// The image is harness-free and agent-agnostic.
pub fn build_dev_image(
    cfg: &ApprovedConfig,
    nix_daemon: bool,
    force: bool,
    no_cache: bool,
) -> Result<()> {
    let docker = DockerClient;
    let user = get_user()?;
    let opts = BuildOptions { force, no_cache };

    if nix_daemon {
        nix_daemon::build(&docker, opts)?;
    }

    image::ensure_dev_image(&docker, cfg, &user, opts)?;

    Ok(())
}
