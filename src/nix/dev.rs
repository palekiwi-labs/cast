use std::fs;
use tempfile::TempDir;

use crate::config::Config;
use crate::nix::dev_image::{get_dockerfile, get_entrypoint, get_image_tag};
use crate::nix::docker::{DockerClient, Result};
use crate::nix::extra_dirs::resolve_extra_dirs;
use crate::user::ResolvedUser;

/// Build the nix dev image locally.
pub fn build_dev<D: DockerClient>(
    docker: &D,
    config: &Config,
    user: &ResolvedUser,
    version: &str,
    force: bool,
    no_cache: bool,
) -> Result<()> {
    let image_tag = get_image_tag(version);

    if !force && docker.image_exists(&image_tag)? {
        println!("Nix dev image already exists: {}", image_tag);
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

    docker.build_image(&image_tag, context_path, &build_args, no_cache)?;

    Ok(())
}
