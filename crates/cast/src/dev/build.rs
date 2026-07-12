use anyhow::Result;

use crate::config::ApprovedConfig;
use crate::dev::agent::Agent;
use crate::dev::universal::image::{ensure_universal_image, universal_image_tag};
use crate::dev::universal::registry::{resolve_included_agents, validate_agent_included};
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::nix_daemon;
use crate::user::get_user;

/// Build an agent's Docker image and optionally the Nix daemon base image.
///
/// In universal mode (`universal_container: true` in config), this builds the
/// combined universal image containing every agent pinned in `agent_versions`
/// instead of a per-agent image.
pub fn build_agent(
    agent: &dyn Agent,
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

    // ── Universal mode: build the combined image ─────────────────────────
    if cfg.universal_container {
        cfg.validate()?;
        validate_agent_included(cfg, agent.name())?;

        let included = resolve_included_agents(cfg);
        let versions: Vec<(&dyn Agent, &str)> = included.iter().map(|(a, v)| (*a, *v)).collect();

        let image_tag = universal_image_tag(&cfg.agent_versions);
        eprintln!("Universal mode: building combined image ({})", image_tag);

        ensure_universal_image(&versions, &docker, cfg, &user, opts)?;

        return Ok(());
    }

    // ── Non-universal path: per-agent image ──────────────────────────────
    let version = agent.resolve_version(cfg)?;
    agent.ensure_image(&docker, cfg, &user, &version, opts)?;

    Ok(())
}
