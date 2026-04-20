use anyhow::Result;

use crate::config::Config;
use crate::dev::build::build_dev;
use crate::dev::container_name::resolve_container_name;
use crate::dev::env_passthrough::build_passthrough_env_args;
use crate::dev::image::get_image_tag;
use crate::dev::workspace::get_workspace;
use crate::docker::args::build_run_args;
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::nix;
use crate::user::get_user;
use crate::version::github::GithubVersionFetcher;
use crate::version::{get_cache_path, resolve_version};

use super::port::calculate_port;

pub fn handle_opencode(config: &Config, extra_args: Vec<String>) -> Result<()> {
    let docker = DockerClient;
    let user = get_user()?;
    let workspace = get_workspace(&user.username)?;

    // Ensure the Nix daemon is running before attempting to start the dev container.
    nix::ensure_running(&docker, config)?;

    // Resolve the concrete opencode version and the corresponding image tag.
    let cache_path = get_cache_path();
    let version = resolve_version(
        &config.opencode_version,
        config.version_cache_ttl_hours,
        &cache_path,
        &GithubVersionFetcher,
    )?;
    let image_tag = get_image_tag(&version);

    // Build the dev image if it does not already exist locally.
    if !docker.image_exists(&image_tag)? {
        println!(
            "Image {} not found, building nix dev environment...",
            image_tag
        );
        build_dev(&docker, config, &user, &version, BuildOptions::default())?;
    }

    // Resolve port and container name.
    let port = match config.port {
        Some(p) => p,
        None => calculate_port()?,
    };
    let cwd_basename = workspace
        .root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("ocx");
    let container_name = resolve_container_name(config, cwd_basename, port);

    // Build docker run options.
    let mut opts: Vec<String> = vec![
        "--rm".to_string(),
        "-it".to_string(),
        // Security hardening
        "--security-opt".to_string(),
        "no-new-privileges".to_string(),
        "--cap-drop".to_string(),
        "ALL".to_string(),
        // Resource constraints
        "--memory".to_string(),
        config.memory.clone(),
        "--cpus".to_string(),
        config.cpus.to_string(),
        "--pids-limit".to_string(),
        config.pids_limit.to_string(),
        // Network
        "--network".to_string(),
        config.network.clone(),
    ];

    // Port publishing.
    if config.publish_port {
        opts.push("-p".to_string());
        opts.push(format!("{}:80", port));
    }

    // Environment: user identity and terminal capabilities.
    opts.extend([
        "-e".to_string(),
        format!("USER={}", user.username),
        "-e".to_string(),
        "TERM=xterm-256color".to_string(),
        "-e".to_string(),
        "COLORTERM=truecolor".to_string(),
        "-e".to_string(),
        "FORCE_COLOR=1".to_string(),
    ]);

    // LLM API keys and OpenCode-specific env vars present on the host.
    opts.extend(build_passthrough_env_args());

    // Workspace bind mount.
    opts.extend([
        "-v".to_string(),
        format!(
            "{}:{}:rw",
            workspace.root.display(),
            workspace.container_path.display()
        ),
        "--workdir".to_string(),
        workspace.container_path.to_string_lossy().into_owned(),
    ]);

    // Build the full docker run argument list and exec into it.
    let mut cmd = config.opencode_command.clone();
    cmd.extend(extra_args);

    let docker_args = build_run_args(&container_name, &image_tag, opts, Some(cmd));

    Err(docker.exec_command(docker_args))
}
