use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;

use crate::config::Config;
use crate::dev;
use crate::dev::agent::Agent;
use crate::dev::container_name::resolve_container_name;
use crate::dev::env_file::build_env_file_args;
use crate::dev::shadow_mounts::{build_shadow_mount_args, resolve_shadow_mounts};
use crate::dev::volumes::build_extra_volume_args;
use crate::dev::workspace::{get_workspace, ResolvedWorkspace};
use crate::docker::args::build_run_args;
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::nix_daemon;
use crate::user::{get_user, ResolvedUser};

/// Generic options for building the Docker run command.
/// Contains only agent-agnostic data; each agent resolves its own
/// additional context inside `Agent::extra_run_args`.
pub struct RunOpts {
    pub workspace: ResolvedWorkspace,
    pub user: ResolvedUser,
    pub port: u16,
    pub host_home_dir: Option<PathBuf>,
    pub user_flake_present: bool,
}

/// Orchestrate and run an agent session inside the dev container.
pub fn run_agent(agent: &dyn Agent, config: &Config, extra_args: Vec<String>) -> Result<()> {
    let docker = DockerClient;
    let user = get_user()?;
    let workspace = get_workspace(&user.username)?;

    // Ensure the Nix daemon is running.
    nix_daemon::ensure_running(&docker, config)?;

    // Resolve the version and image for this agent, and ensure it exists locally.
    let version = agent.resolve_version(config)?;
    let image_tag = agent.image_tag(&version);
    agent.ensure_image(&docker, config, &user, &version, BuildOptions::default())?;

    // Resolve port and container name.
    let port = dev::port::resolve_port(config)?;
    let cwd_basename = workspace.root_basename();
    let container_name = resolve_container_name(config, agent.name(), cwd_basename, port);

    let host_home_dir = dirs::home_dir();
    let env: HashMap<String, String> = std::env::vars().collect();

    let user_flake_present = host_home_dir
        .as_ref()
        .filter(|h| h.join(".config/cast/nix/flake.nix").exists())
        .is_some();

    let run_opts = RunOpts {
        workspace,
        user,
        port,
        host_home_dir,
        user_flake_present,
    };

    // Prepare host-side side effects before building arguments.
    agent.prepare_host(config, &run_opts)?;

    // Build generic docker run flags, then append agent-specific ones.
    let mut opts = build_run_opts(config, &run_opts);
    opts.extend(agent.extra_run_args(config, &run_opts, &env)?);

    // Build the full command and exec into the container.
    let cmd = agent.build_command(config, &run_opts, extra_args);
    let docker_args = build_run_args(&container_name, &image_tag, opts, Some(cmd));
    Err(docker.exec_command(docker_args))
}

/// Build the generic set of Docker run flags that apply to every agent.
///
/// Agent-specific arguments (env vars, program-specific mounts, etc.) are
/// NOT included here — each agent appends them via `Agent::extra_run_args`.
pub fn build_run_opts(config: &Config, opts: &RunOpts) -> Vec<String> {
    let mut run_args: Vec<String> = vec![
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
        run_args.push("-p".to_string());
        run_args.push(format!("{}:80", opts.port));
    }

    // Env files.
    run_args.extend(build_env_file_args(
        &opts.workspace.root,
        opts.host_home_dir.as_deref(),
    ));

    // Environment: user identity and terminal capabilities.
    run_args.extend([
        "-e".to_string(),
        format!("USER={}", opts.user.username),
        "-e".to_string(),
        "TERM=xterm-256color".to_string(),
        "-e".to_string(),
        "COLORTERM=truecolor".to_string(),
        "-e".to_string(),
        "FORCE_COLOR=1".to_string(),
    ]);

    // Nix store.
    run_args.extend([
        "-v".to_string(),
        format!("{}:/nix:ro", config.nix_volume_name),
    ]);

    // Timezone.
    run_args.extend([
        "-v".to_string(),
        "/etc/localtime:/etc/localtime:ro".to_string(),
    ]);

    // Workspace bind mount.
    run_args.extend([
        "-v".to_string(),
        format!(
            "{}:{}:rw",
            opts.workspace.root.display(),
            opts.workspace.container_path.display()
        ),
    ]);

    // Data volumes.
    run_args.extend(build_extra_volume_args(
        config,
        &opts.user,
        &opts.workspace,
        opts.host_home_dir.as_deref(),
    ));

    // Shadow mounts.
    let shadow_mounts = resolve_shadow_mounts(&config.forbidden_paths, &opts.workspace);
    run_args.extend(build_shadow_mount_args(&shadow_mounts));

    // Working directory.
    run_args.push("--workdir".to_string());
    run_args.push(opts.workspace.container_path.to_string_lossy().into_owned());

    // Host internal networking
    if config.add_host_docker_internal {
        run_args.push("--add-host".to_string());
        run_args.push("host.docker.internal:host-gateway".to_string());
    }

    run_args
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_build_run_opts_basic() {
        let config = Config::default();
        let user = ResolvedUser {
            username: "alice".to_string(),
            uid: 1000,
            gid: 1000,
        };
        let workspace = ResolvedWorkspace {
            root: PathBuf::from("/home/alice/project"),
            container_path: PathBuf::from("/home/alice/project"),
        };

        let opts = RunOpts {
            workspace,
            user,
            port: 32768,
            host_home_dir: Some(PathBuf::from("/home/alice")),
            user_flake_present: false,
        };

        let run_args = build_run_opts(&config, &opts);

        // Generic flags present
        assert!(run_args.contains(&"--rm".to_string()));
        assert!(run_args.contains(&"-it".to_string()));
        assert!(run_args.contains(&"no-new-privileges".to_string()));
        assert!(run_args.contains(&"USER=alice".to_string()));
        assert!(run_args.contains(&"/home/alice/project:/home/alice/project:rw".to_string()));

        // Nix store and timezone
        assert!(run_args.contains(&format!("{}:/nix:ro", config.nix_volume_name)));
        assert!(run_args.contains(&"/etc/localtime:/etc/localtime:ro".to_string()));
        assert!(run_args.contains(&"--workdir".to_string()));

        // Data volumes must NOT be present by default (agent-specific)
        assert!(!run_args.iter().any(|a| a.contains("cast-cache")));
        assert!(!run_args.iter().any(|a| a.contains("cast-local")));

        // OpenCode-specific args must NOT be present
        assert!(!run_args.iter().any(|a| a.contains("opencode")));
        assert!(!run_args.iter().any(|a| a.contains("cast/nix")));
    }

    #[test]
    fn test_build_run_opts_add_host_enabled() {
        let config = Config {
            add_host_docker_internal: true,
            ..Config::default()
        };
        let user = ResolvedUser {
            username: "alice".to_string(),
            uid: 1000,
            gid: 1000,
        };
        let workspace = ResolvedWorkspace {
            root: PathBuf::from("/home/alice/project"),
            container_path: PathBuf::from("/home/alice/project"),
        };
        let opts = RunOpts {
            workspace,
            user,
            port: 32768,
            host_home_dir: Some(PathBuf::from("/home/alice")),
            user_flake_present: false,
        };

        let run_args = build_run_opts(&config, &opts);

        let add_host_pos = run_args.iter().position(|r| r == "--add-host");
        assert!(add_host_pos.is_some(), "Should contain --add-host");
        assert_eq!(
            run_args[add_host_pos.unwrap() + 1],
            "host.docker.internal:host-gateway"
        );
    }

    #[test]
    fn test_build_run_opts_add_host_disabled() {
        let config = Config {
            add_host_docker_internal: false,
            ..Config::default()
        };
        let user = ResolvedUser {
            username: "alice".to_string(),
            uid: 1000,
            gid: 1000,
        };
        let workspace = ResolvedWorkspace {
            root: PathBuf::from("/home/alice/project"),
            container_path: PathBuf::from("/home/alice/project"),
        };
        let opts = RunOpts {
            workspace,
            user,
            port: 32768,
            host_home_dir: Some(PathBuf::from("/home/alice")),
            user_flake_present: false,
        };

        let run_args = build_run_opts(&config, &opts);

        assert!(
            !run_args.contains(&"--add-host".to_string()),
            "Should NOT contain --add-host when disabled"
        );
    }

    #[test]
    fn test_build_run_opts_shadow_mounts() {
        let config = Config {
            forbidden_paths: vec!["secrets".to_string()],
            ..Config::default()
        };

        let user = ResolvedUser {
            username: "alice".to_string(),
            uid: 1000,
            gid: 1000,
        };

        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        std::fs::create_dir(root.join("secrets")).unwrap();

        let workspace = ResolvedWorkspace {
            root,
            container_path: PathBuf::from("/home/alice/project"),
        };

        let opts = RunOpts {
            workspace,
            user,
            port: 32768,
            host_home_dir: Some(PathBuf::from("/home/alice")),
            user_flake_present: false,
        };

        let run_args = build_run_opts(&config, &opts);

        assert!(run_args.contains(
            &"/home/alice/project/secrets:ro,noexec,nosuid,size=1k,mode=000".to_string()
        ));
    }
}
