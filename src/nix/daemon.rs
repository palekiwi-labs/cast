use std::fs;
use tempfile::TempDir;

use crate::config::Config;
use crate::nix::docker::{DockerClient, Result};
use crate::nix::{config as nix_config, image};

/// Ensure the nix daemon container is running
pub fn ensure_running<D: DockerClient>(docker: &D, config: &Config) -> Result<()> {
    let container_name = &config.nix_daemon_container_name;

    // Check if already running
    if docker.is_container_running(container_name)? {
        println!("Nix daemon is already running: {}", container_name);
        return Ok(());
    }

    // Get the dynamic image tag based on assets hash
    let image_tag = image::get_image_tag();

    // Check if the image exists, build it if it doesn't
    if !docker.image_exists(&image_tag)? {
        println!("Building nix daemon image: {}", image_tag);
        build_image(docker, &image_tag)?;
    }

    // Generate dynamic nix.conf content
    let nix_conf_content = nix_config::generate_nix_conf(config);

    // Start the daemon container
    println!("Starting nix daemon container: {}", container_name);

    let volume_mount = format!("{}:/nix:rw", &config.nix_volume_name);
    let volumes = vec![volume_mount.as_str()];

    // Pass configuration via environment variable
    let env_vars = vec![("NIX_CONF_CONTENT", nix_conf_content.as_str())];

    docker.run_container(
        container_name,
        &image_tag,
        &volumes,
        &env_vars,
        true, // detached
        true, // remove on stop
    )?;

    println!("Nix daemon started successfully");
    Ok(())
}

/// Build the custom nix daemon image
fn build_image<D: DockerClient>(docker: &D, tag: &str) -> Result<()> {
    // Create a temporary directory for the build context
    let temp_dir = TempDir::new()?;
    let context_path = temp_dir.path();

    // Write the Dockerfile
    let dockerfile_path = context_path.join("Dockerfile");
    fs::write(&dockerfile_path, image::get_dockerfile())?;

    // Write the entrypoint script
    let entrypoint_path = context_path.join("entrypoint.sh");
    fs::write(&entrypoint_path, image::get_entrypoint())?;

    // Build the image
    docker.build_image(tag, context_path)?;

    Ok(())
}

/// Explicitly build the custom nix daemon image
pub fn build<D: DockerClient>(docker: &D) -> Result<()> {
    let image_tag = image::get_image_tag();
    println!("Building nix daemon image: {}", image_tag);
    build_image(docker, &image_tag)
}

/// Stop the nix daemon container
pub fn stop<D: DockerClient>(docker: &D, config: &Config) -> Result<()> {
    let container_name = &config.nix_daemon_container_name;

    // Check if it's actually running
    if !docker.is_container_running(container_name)? {
        println!("Nix daemon is not running: {}", container_name);
        return Ok(());
    }

    println!("Stopping nix daemon container: {}", container_name);
    docker.stop_container(container_name)?;
    println!("Nix daemon stopped successfully");

    Ok(())
}
