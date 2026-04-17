use crate::config::Config;
use crate::nix::docker::{DockerClient, Result};

/// Default Docker image for the nix daemon
const NIX_DAEMON_IMAGE: &str = "nixos/nix:latest";

/// Ensure the nix daemon container is running
pub fn ensure_running<D: DockerClient>(docker: &D, config: &Config) -> Result<()> {
    let container_name = &config.nix_daemon_container_name;

    // Check if already running
    if docker.is_container_running(container_name)? {
        println!("Nix daemon is already running: {}", container_name);
        return Ok(());
    }

    // Start the daemon container
    println!("Starting nix daemon container: {}", container_name);

    let volume_mount = format!("{}:/nix:rw", &config.nix_volume_name);
    let volumes = vec![volume_mount.as_str()];

    docker.run_container(
        container_name,
        NIX_DAEMON_IMAGE,
        &volumes,
        true, // detached
        true, // remove on stop
    )?;

    println!("Nix daemon started successfully");
    Ok(())
}
