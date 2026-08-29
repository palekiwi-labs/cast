use std::collections::BTreeSet;

use anyhow::Result;

use crate::config::{ApprovedConfig, Config};
use crate::dev::build_command::build_sandbox_command;
use crate::dev::run::{build_service_run_flags, resolve_run_opts, RunMode, RunOpts, SessionFlags};
use crate::dev::service_context::ServiceContext;
use crate::docker::args::build_run_args;
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::user::get_user;
use crate::{dev, nix_daemon};

pub fn build_service_command(config: &Config, username: &str) -> Vec<String> {
    build_sandbox_command(config, username, "herdr", vec!["server".to_string()])
}

pub fn build_service_docker_args(
    config: &Config,
    opts: &RunOpts,
    context: &ServiceContext,
    service_name: Option<&str>,
    image_tag: &str,
    host_env_names: &BTreeSet<String>,
) -> Vec<String> {
    let container_name = context.container_name(service_name);
    let multiplexer_session = context.multiplexer_session_name(service_name);
    let container_workdir = context.container_workdir(&opts.workspace);
    let flags = build_service_run_flags(
        config,
        opts,
        host_env_names,
        &multiplexer_session,
        &context.git_common_dir,
        &container_workdir,
    );
    let command = build_service_command(config, &opts.user.username);

    build_run_args(&container_name, image_tag, flags, Some(command))
}

pub fn up(
    config: &ApprovedConfig,
    context: &ServiceContext,
    service_name: Option<&str>,
) -> Result<()> {
    let docker = DockerClient;
    let container_name = context.container_name(service_name);
    if docker.is_container_running(&container_name)? {
        eprintln!("service is already running: {container_name}");
        return Ok(());
    }

    let user = get_user()?;
    let workspace = context.workspace(dirs::home_dir().as_deref(), &user.username);
    let run_opts = resolve_run_opts(
        user,
        workspace,
        0,
        &SessionFlags {
            mode: RunMode::Headless {
                token: "service".to_string(),
            },
            name: service_name.map(str::to_string),
            publish: false,
        },
    );

    nix_daemon::ensure_running(&docker, config)?;
    dev::image::ensure_dev_image(&docker, config, &run_opts.user, BuildOptions::default())?;

    let host_env_names = std::env::vars()
        .filter(|(_, value)| !value.is_empty())
        .map(|(name, _)| name)
        .collect::<BTreeSet<_>>();
    let image_tag = dev::image::image_tag();
    let args = build_service_docker_args(
        config,
        &run_opts,
        context,
        service_name,
        &image_tag,
        &host_env_names,
    );
    docker.run_command(args)?;
    eprintln!("service started: {container_name}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dev::run::{RunOpts, TtyMode};
    use crate::dev::service_context::ServiceContext;
    use crate::dev::workspace::ResolvedWorkspace;
    use crate::user::ResolvedUser;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    #[test]
    fn service_command_starts_herdr_in_the_sandbox_shell_only() {
        let config = Config {
            sandbox_shell: Some("~/.config/cast/nix#default".to_string()),
            project_shell: Some(".#project".to_string()),
            ..Default::default()
        };

        let command = build_service_command(&config, "alice");

        assert_eq!(
            command,
            vec![
                "nix",
                "develop",
                "/home/alice/.config/cast/nix#default",
                "-c",
                "herdr",
                "server",
            ]
        );
    }

    #[test]
    fn service_command_is_bare_without_a_sandbox_shell() {
        let config = Config {
            project_shell: Some(".#project".to_string()),
            ..Default::default()
        };

        let command = build_service_command(&config, "alice");

        assert_eq!(command, vec!["herdr", "server"]);
    }

    #[test]
    fn service_docker_args_target_the_named_worktree_service() {
        let config = Config::default();
        let context = ServiceContext {
            worktree_root: PathBuf::from("/home/alice/projects/my-app"),
            git_common_dir: PathBuf::from("/home/alice/main/.git"),
            relative_cwd: PathBuf::from("crates/app"),
            workspace_id: "a1b2c3d4e5f6".to_string(),
        };
        let workspace = ResolvedWorkspace {
            root: context.worktree_root.clone(),
            container_path: PathBuf::from("/home/alice/projects/my-app"),
        };
        let opts = RunOpts {
            workspace,
            user: ResolvedUser {
                username: "alice".to_string(),
                uid: 1000,
                gid: 1000,
            },
            port: 0,
            host_home_dir: None,
            host_name: "test-host".to_string(),
            tty_mode: TtyMode::Headless,
            publish: false,
        };

        let args = build_service_docker_args(
            &config,
            &opts,
            &context,
            Some("isolated"),
            "localhost/cast:spike",
            &BTreeSet::new(),
        );

        assert_eq!(
            &args[..3],
            ["run", "--name", "cast-my-app-a1b2c3d4e5f6-isolated"]
        );
        assert!(args.contains(&"HERDR_SESSION=cast-a1b2c3d4e5f6-isolated".to_string()));
        assert!(args.ends_with(&[
            "localhost/cast:spike".to_string(),
            "herdr".to_string(),
            "server".to_string()
        ]));
    }
}
