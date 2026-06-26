use std::process::ExitStatus;

use anyhow::{bail, Result};

use crate::config::ApprovedConfig;
use crate::dev::agent::Agent;
use crate::dev::build_command::build_command;
use crate::dev::container_name::resolve_container_name;
use crate::dev::port::resolve_port;
use crate::dev::run::{resolve_run_opts, SessionFlags};
use crate::dev::workspace::get_workspace;
use crate::docker::client::DockerClient;
use crate::user::get_user;

/// Drop into an interactive shell in the dev container
pub fn shell(agent: &dyn Agent, config: &ApprovedConfig, raw: bool) -> Result<ExitStatus> {
    let docker = DockerClient;
    let user = get_user()?;
    let workspace = get_workspace(&user.username)?;
    let port = resolve_port(config, agent.name())?;

    let cwd_basename = workspace.root_basename();
    let container_name =
        resolve_container_name(config, agent.name(), cwd_basename, port, None, None);

    if !docker.is_container_running(&container_name)? {
        bail!(
            "Dev container is not running: {}. Run 'ocx run {}' to start it.",
            container_name,
            agent.name(),
        );
    }

    let mut exec_args = vec!["exec".to_string(), "-it".to_string(), container_name];

    let shell_cmd = if raw {
        vec!["/bin/bash".to_string()]
    } else {
        let flags = SessionFlags {
            mode: crate::dev::run::RunMode::Interactive,
            name: None,
            publish: None,
        };
        let opts = resolve_run_opts(user, workspace, port, &flags);
        build_command(config, &opts, "/bin/bash", vec![])
    };

    exec_args.extend(shell_cmd);

    docker.interactive_command(exec_args)
}
