use std::fs;
use tempfile::TempDir;

use crate::config::Config;
use crate::dev::extra_dirs::resolve_extra_dirs;
use crate::docker::args;
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::user::ResolvedUser;
use anyhow::Result;

const DOCKERFILE: &str = include_str!("../../../assets/Dockerfile.dev.opencode");
const IMAGE_BASE: &str = "localhost/cast";
const CAST_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Get the full image tag: `localhost/cast:{cast_version}-opencode-{opencode_version}`
pub fn get_image_tag(opencode_version: &str) -> String {
    format!(
        "{}:{}-opencode-{}",
        IMAGE_BASE, CAST_VERSION, opencode_version
    )
}

/// Get the embedded Dockerfile content for the nix dev image.
pub fn get_dockerfile() -> &'static str {
    DOCKERFILE
}

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
    opts: BuildOptions,
) -> Result<()> {
    let image_tag = get_image_tag(version);

    if !docker.image_exists(&image_tag)? {
        println!(
            "Image {} not found, building nix dev environment...",
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
            get_image_tag("1.4.7"),
            format!(
                "localhost/cast:{}-opencode-1.4.7",
                env!("CARGO_PKG_VERSION")
            )
        );
    }

    #[test]
    fn test_get_dockerfile_has_correct_base_image() {
        assert!(get_dockerfile().contains("FROM debian:trixie-slim"));
    }
}
