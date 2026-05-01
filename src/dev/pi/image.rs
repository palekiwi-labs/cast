use std::fs;
use tempfile::TempDir;

use crate::config::Config;
use crate::dev::extra_dirs::resolve_extra_dirs;
use crate::docker::args;
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::user::ResolvedUser;
use anyhow::Result;

const DOCKERFILE: &str = include_str!("../../../assets/Dockerfile.dev.pi");
const IMAGE_BASE: &str = "localhost/cast";
const CAST_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Get the full image tag: `localhost/cast:{cast_version}-pi-{pi_version}`
pub fn get_image_tag(pi_version: &str) -> String {
    format!("{}:{}-pi-{}", IMAGE_BASE, CAST_VERSION, pi_version)
}

/// Get the embedded Dockerfile content for the pi dev image.
pub fn get_dockerfile() -> &'static str {
    DOCKERFILE
}

/// Build the pi dev image locally.
pub fn build_dev(
    docker: &DockerClient,
    config: &Config,
    user: &ResolvedUser,
    version: &str,
    opts: BuildOptions,
) -> Result<()> {
    let image_tag = get_image_tag(version);

    if !opts.force && docker.image_exists(&image_tag)? {
        println!("Pi dev image already exists: {}", image_tag);
        if opts.no_cache {
            println!("Hint: You passed --no-cache. If you want to force a rebuild of the existing image, use --force.");
        }
        return Ok(());
    }

    println!("Building pi dev image: {}", image_tag);

    let temp_dir = TempDir::new()?;
    let context_path = temp_dir.path();

    let dockerfile_path = context_path.join("Dockerfile");
    fs::write(&dockerfile_path, get_dockerfile())?;

    let extra_dirs = resolve_extra_dirs(config, &user.username);
    let uid_str = user.uid.to_string();
    let gid_str = user.gid.to_string();

    let build_args = [
        ("PI_VERSION", version),
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
    opts: BuildOptions,
) -> Result<()> {
    let image_tag = get_image_tag(version);

    if !docker.image_exists(&image_tag)? {
        println!(
            "Image {} not found, building pi dev environment...",
            image_tag
        );
        build_dev(docker, config, user, version, opts)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_image_tag_format() {
        assert_eq!(
            get_image_tag("0.71.0"),
            format!("localhost/cast:{}-pi-0.71.0", env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn test_get_dockerfile_has_correct_base_image() {
        assert!(get_dockerfile().contains("FROM debian:trixie-slim"));
    }
}
