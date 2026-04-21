use std::fs;
use tempfile::TempDir;

use crate::config::Config;
use crate::docker::args;
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::user::ResolvedUser;
use anyhow::Result;

use super::extra_dirs::resolve_extra_dirs;
use super::image::{get_dockerfile, get_entrypoint, get_image_tag};

/// Build the nix dev image locally.
pub fn build_dev(
    docker: &DockerClient,
    config: &Config,
    user: &ResolvedUser,
    version: &str,
    opts: BuildOptions,
) -> Result<()> {
    let image_tag = get_image_tag(version);

    if !opts.force && docker.image_exists(&image_tag)? {
        println!("Nix dev image already exists: {}", image_tag);
        if opts.no_cache {
            println!("Hint: You passed --no-cache. If you want to force a rebuild of the existing image, use --force.");
        }
        return Ok(());
    }

    println!("Building nix dev image: {}", image_tag);

    let temp_dir = TempDir::new()?;
    let context_path = temp_dir.path();

    let dockerfile_path = context_path.join("Dockerfile");
    fs::write(&dockerfile_path, get_dockerfile())?;

    let entrypoint_path = context_path.join("entrypoint.sh");
    fs::write(&entrypoint_path, get_entrypoint())?;

    let extra_dirs = resolve_extra_dirs(config, &user.username);
    let uid_str = user.uid.to_string();
    let gid_str = user.gid.to_string();

    let build_args = [
        ("OPENCODE_VERSION", version),
        ("USERNAME", &user.username),
        ("UID", &uid_str),
        ("GID", &gid_str),
        ("EXTRA_DIRS", &extra_dirs),
    ];

    let docker_build_args =
        args::build_docker_build_args(&image_tag, context_path, &build_args, opts.no_cache);
    docker.stream_command(docker_build_args)?;

    Ok(())
}

/// Ensure the dev image exists locally, building it if necessary.
pub fn ensure_dev_image(
    docker: &DockerClient,
    config: &Config,
    user: &ResolvedUser,
    version: &str,
) -> Result<()> {
    let image_tag = get_image_tag(version);

    if !docker.image_exists(&image_tag)? {
        println!(
            "Image {} not found, building nix dev environment...",
            image_tag
        );
        build_dev(docker, config, user, version, BuildOptions::default())?;
    }

    Ok(())
}
