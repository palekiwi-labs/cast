use anyhow::{Context, Result};

use crate::dev::run::RunOpts;

pub mod config_dir;
pub mod registry;
pub mod volumes;

/// Prepare host-side state shared across every agent (universal mounts).
///
/// Currently ensures the cross-harness `~/.agents` config directory exists on
/// the host with correct user ownership before the container starts, so a
/// missing directory does not get auto-created by Docker as a root-owned path
/// when the bind mount is attached.
pub fn prepare_host(opts: &RunOpts) -> Result<()> {
    let home = opts
        .host_home_dir
        .as_deref()
        .context("Failed to resolve user home directory")?;
    config_dir::ensure_config_dir(home)?;
    Ok(())
}
