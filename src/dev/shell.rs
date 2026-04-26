use anyhow::Result;

use crate::config::Config;
use crate::dev::container_name::resolve_container_name;
use crate::dev::port::resolve_port;
use crate::dev::workspace::get_workspace;
use crate::docker::client::DockerClient;
use crate::user::get_user;

/// Drop into an interactive shell in the dev container
pub fn shell(config: &Config) -> Result<()> {
    let docker = DockerClient;
    let user = get_user()?;
    let workspace = get_workspace(&user.username)?;
    let port = resolve_port(config)?;

    let cwd_basename = workspace.root_basename();

    let container_name = resolve_container_name(config, cwd_basename, port);

    if !docker.is_container_running(&container_name)? {
        println!(
            "Dev container is not running: {}. Run 'ocx opencode' to start it.",
            container_name
        );
        return Ok(());
    }

    let exec_args = vec![
        "exec".to_string(),
        "-it".to_string(),
        container_name,
        "/bin/bash".to_string(),
    ];

    Err(docker.exec_command(exec_args))
}
