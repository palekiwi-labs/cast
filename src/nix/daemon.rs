use std::fs;
use tempfile::TempDir;

use crate::config::Config;
use crate::docker::args;
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::nix::{config as nix_config, image};
use anyhow::Result;

/// Ensure the nix daemon container is running
pub fn ensure_running(docker: &DockerClient, config: &Config) -> Result<()> {
    let container_name = &config.nix_daemon_container_name;

    // Check if already running
    if docker.is_container_running(container_name)? {
        return Ok(());
    }

    // Get the dynamic image tag based on assets hash
    let image_tag = image::get_image_tag();

    // Check if the image exists, build it if it doesn't
    if !docker.image_exists(&image_tag)? {
        println!("Building nix daemon image: {}", image_tag);
        build_image(docker, &image_tag, false)?;
    }

    // Generate dynamic nix.conf content
    let nix_conf_content = nix_config::generate_nix_conf(config);

    // Start the daemon container
    println!("Starting nix daemon container: {}", container_name);

    // Assemble options
    let opts = vec![
        "-d".to_string(),
        "--rm".to_string(),
        "-e".to_string(),
        format!("NIX_CONF_CONTENT={}", nix_conf_content),
        "-v".to_string(),
        format!("{}:/nix:rw", config.nix_volume_name),
    ];

    let run_args = args::build_run_args(container_name, &image_tag, opts, None);
    docker.run_command(run_args)?;

    println!("Nix daemon started successfully");
    Ok(())
}

/// Build the custom nix daemon image
fn build_image(docker: &DockerClient, tag: &str, no_cache: bool) -> Result<()> {
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
    let build_args = args::build_docker_build_args(tag, context_path, &[], no_cache);
    docker.stream_command(build_args)?;

    Ok(())
}

/// Explicitly build the custom nix daemon image
pub fn build(docker: &DockerClient, opts: BuildOptions) -> Result<()> {
    let image_tag = image::get_image_tag();

    if !opts.force && docker.image_exists(&image_tag)? {
        println!("Nix daemon image already exists: {}", image_tag);
        return Ok(());
    }

    println!("Building nix daemon image: {}", image_tag);
    build_image(docker, &image_tag, opts.no_cache)
}

/// Stop the nix daemon container
pub fn stop(docker: &DockerClient, config: &Config) -> Result<()> {
    let container_name = &config.nix_daemon_container_name;

    // Check if it's actually running
    if !docker.is_container_running(container_name)? {
        println!("Nix daemon is not running: {}", container_name);
        return Ok(());
    }

    println!("Stopping nix daemon container: {}", container_name);
    let stop_args = args::build_stop_args(container_name);
    docker.run_command(stop_args)?;
    println!("Nix daemon stopped successfully");

    Ok(())
}
