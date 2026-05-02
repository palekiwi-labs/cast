use std::fs;
use tempfile::TempDir;

use crate::config::Config;
use crate::dev::extra_dirs::resolve_extra_dirs;
use crate::docker::args;
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::user::ResolvedUser;
use anyhow::Result;

const IMAGE_BASE: &str = "localhost/cast";
const CAST_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Get the full image tag for an agent.
pub fn image_tag(agent_name: &str, version: &str) -> String {
    format!("{}:{}-{}-{}", IMAGE_BASE, CAST_VERSION, agent_name, version)
}

/// Ensure an agent image exists locally, building it if necessary.
pub fn ensure_image(
    agent_name: &str,
    dockerfile: &str,
    docker: &DockerClient,
    config: &Config,
    user: &ResolvedUser,
    version: &str,
    opts: BuildOptions,
) -> Result<()> {
    let image_tag = image_tag(agent_name, version);

    if !opts.force && docker.image_exists(&image_tag)? {
        println!("{} dev image already exists: {}", agent_name, image_tag);
        if opts.no_cache {
            println!("Hint: You passed --no-cache. If you want to force a rebuild of the existing image, use --force.");
        }
        return Ok(());
    }

    println!("Building {} dev image: {}", agent_name, image_tag);

    let temp_dir = TempDir::new()?;
    let context_path = temp_dir.path();

    let dockerfile_path = context_path.join("Dockerfile");
    fs::write(&dockerfile_path, dockerfile)?;

    let extra_dirs = resolve_extra_dirs(config, &user.username);
    let uid_str = user.uid.to_string();
    let gid_str = user.gid.to_string();

    let build_args = [
        ("AGENT_VERSION", version),
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
