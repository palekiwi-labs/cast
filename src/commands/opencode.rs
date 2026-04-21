use anyhow::Result;

use crate::config::Config;
use crate::dev;
use crate::dev::container_name::resolve_container_name;
use crate::dev::workspace::get_workspace;
use crate::docker::args::build_run_args;
use crate::docker::client::DockerClient;
use crate::nix;
use crate::user::get_user;

pub fn handle_opencode(config: &Config, extra_args: Vec<String>) -> Result<()> {
    let docker = DockerClient;
    let user = get_user()?;
    let workspace = get_workspace(&user.username)?;

    // Ensure the Nix daemon is running.
    nix::ensure_running(&docker, config)?;

    // Resolve version and ensure the dev image exists.
    let version = dev::resolve_opencode_version(config)?;
    let image_tag = dev::image::get_image_tag(&version);
    dev::ensure_dev_image(&docker, config, &user, &version)?;

    // Resolve port and container name.
    let port = dev::port::resolve_port(config)?;
    let cwd_basename = workspace
        .root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("ocx");
    let container_name = resolve_container_name(config, cwd_basename, port);

    // Build docker run flags.
    let opts = dev::run::build_run_opts(config, &user, &workspace, port);

    // Build the full command.
    let mut cmd = config.opencode_command.clone();
    cmd.extend(extra_args);

    // Exec into the container.
    let docker_args = build_run_args(&container_name, &image_tag, opts, Some(cmd));
    Err(docker.exec_command(docker_args))
}
