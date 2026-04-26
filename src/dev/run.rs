use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::dev;
use crate::dev::container_name::resolve_container_name;
use crate::dev::env_file::build_env_file_args;
use crate::dev::env_passthrough::build_passthrough_env_args;
use crate::dev::opencode_cmd::resolve_opencode_command;
use crate::dev::shadow_mounts::{build_shadow_mount_args, resolve_shadow_mounts};
use crate::dev::volumes::{build_data_volume_args, build_extra_volume_args};
use crate::dev::workspace::{get_workspace, ResolvedWorkspace};
use crate::docker::args::build_run_args;
use crate::docker::client::DockerClient;
use crate::nix_daemon;
use crate::opencode;
use crate::user::{get_user, ResolvedUser};

/// Options for building the Docker run command.
pub struct RunOpts {
    pub workspace: ResolvedWorkspace,
    pub user: ResolvedUser,
    pub port: u16,
    pub opencode_config_dir: PathBuf,
    pub host_home_dir: Option<PathBuf>,
    pub user_flake_host_dir: Option<PathBuf>,
}

/// Resolves the OPENCODE_CONFIG_DIR environment variable to an absolute host path,
/// expanding leading tildes if necessary, and filtering on path existence.
pub fn resolve_config_dir_env(env_val: Option<String>, home_dir: Option<&Path>) -> Option<PathBuf> {
    env_val
        .map(|p| dev::utils::expand_tilde(&p, home_dir))
        .filter(|p| p.exists())
}

/// Orchestrate and run an OpenCode session.
pub fn run_opencode(config: &Config, extra_args: Vec<String>) -> Result<()> {
    let docker = DockerClient;
    let user = get_user()?;
    let workspace = get_workspace(&user.username)?;

    // Ensure the Nix daemon is running.
    nix_daemon::ensure_running(&docker, config)?;

    // Resolve version and ensure the dev image exists.
    let version = opencode::resolve_version(config)?;
    let image_tag = dev::image::get_image_tag(&version);
    dev::ensure_dev_image(&docker, config, &user, &version)?;

    // Resolve port and container name.
    let port = dev::port::resolve_port(config)?;
    let opencode_config_dir = opencode::ensure_config_dir()?;
    let cwd_basename = workspace.root_basename();
    let container_name = resolve_container_name(config, cwd_basename, port);

    let host_home_dir = dirs::home_dir();
    let user_flake_host_dir = host_home_dir
        .as_ref()
        .filter(|h| h.join(".config/ocx/nix/flake.nix").exists())
        .map(|h| h.join(".config/ocx/nix"));

    let run_opts = RunOpts {
        workspace,
        user,
        port,
        opencode_config_dir,
        host_home_dir,
        user_flake_host_dir,
    };

    // Build docker run flags.
    let opts = build_run_opts(config, &run_opts, std::env::var("OPENCODE_CONFIG_DIR").ok());

    // Build the full command.
    let mut cmd = resolve_opencode_command(
        config,
        &run_opts.user,
        run_opts.user_flake_host_dir.is_some(),
    );
    cmd.extend(extra_args);

    // Exec into the container.
    let docker_args = build_run_args(&container_name, &image_tag, opts, Some(cmd));
    Err(docker.exec_command(docker_args))
}

/// Build the full set of Docker run flags for an OpenCode session.
pub fn build_run_opts(
    config: &Config,
    opts: &RunOpts,
    opencode_config_dir_env_val: Option<String>,
) -> Vec<String> {
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

    // LLM API keys and OpenCode-specific env vars present on the host.
    run_args.extend(build_passthrough_env_args());

    let opencode_config_dir_env =
        resolve_config_dir_env(opencode_config_dir_env_val, opts.host_home_dir.as_deref());

    // OPENCODE_CONFIG_DIR special case: bind-mount with container path rewrite.
    if let Some(config_dir_env) = &opencode_config_dir_env {
        run_args.extend([
            "-v".to_string(),
            format!("{}:/opencode-config-dir:ro", config_dir_env.display()),
            "-e".to_string(),
            "OPENCODE_CONFIG_DIR=/opencode-config-dir".to_string(),
        ]);
    }

    // Nix store.
    run_args.extend([
        "-v".to_string(),
        format!("{}:/nix:ro", config.nix_volume_name),
    ]);

    // User flake mount.
    if let Some(flake_dir) = &opts.user_flake_host_dir {
        run_args.extend([
            "-v".to_string(),
            format!(
                "{}:/home/{}/.config/ocx/nix:rw",
                flake_dir.display(),
                opts.user.username
            ),
        ]);
    }

    // OpenCode config directory bind mount.
    // Skip if the workspace root is the same as the config dir (workspace mount covers it).
    if opts.opencode_config_dir != opts.workspace.root {
        run_args.extend([
            "-v".to_string(),
            format!(
                "{}:/home/{}/.config/opencode:rw",
                opts.opencode_config_dir.display(),
                opts.user.username
            ),
        ]);
    }

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
    run_args.extend(build_data_volume_args(config, &opts.user));
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
        let opencode_config_dir = PathBuf::from("/home/alice/.config/opencode");

        let opts = RunOpts {
            workspace,
            user,
            port: 32768,
            opencode_config_dir,
            host_home_dir: Some(PathBuf::from("/home/alice")),
            user_flake_host_dir: None,
        };

        let run_args = build_run_opts(&config, &opts, None);

        // Check for key flags
        assert!(run_args.contains(&"--rm".to_string()));
        assert!(run_args.contains(&"-it".to_string()));
        assert!(run_args.contains(&"no-new-privileges".to_string()));
        assert!(run_args.contains(&"USER=alice".to_string()));
        assert!(run_args.contains(&"/home/alice/project:/home/alice/project:rw".to_string()));
        assert!(run_args
            .contains(&"/home/alice/.config/opencode:/home/alice/.config/opencode:rw".to_string()));

        // Phase 2 volume checks
        assert!(run_args.contains(&format!("{}:/nix:ro", config.nix_volume_name)));
        assert!(run_args.contains(&"/etc/localtime:/etc/localtime:ro".to_string()));
        assert!(run_args.contains(&"--workdir".to_string()));

        // Data volumes should be present by default
        assert!(run_args.contains(&"ocx-cache:/home/alice/.cache:rw".to_string()));
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
        let opencode_config_dir = PathBuf::from("/home/alice/.config/opencode");

        let opts = RunOpts {
            workspace,
            user,
            port: 32768,
            opencode_config_dir,
            host_home_dir: Some(PathBuf::from("/home/alice")),
            user_flake_host_dir: None,
        };

        let run_args = build_run_opts(&config, &opts, None);

        assert!(run_args.contains(
            &"/home/alice/project/secrets:ro,noexec,nosuid,size=1k,mode=000".to_string()
        ));
    }

    #[test]
    fn test_build_run_opts_user_flake_present() {
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
        let opencode_config_dir = PathBuf::from("/home/alice/.config/opencode");
        let flake_dir = PathBuf::from("/home/alice/.config/ocx/nix");

        let opts = RunOpts {
            workspace,
            user,
            port: 32768,
            opencode_config_dir,
            host_home_dir: Some(PathBuf::from("/home/alice")),
            user_flake_host_dir: Some(flake_dir),
        };

        let run_args = build_run_opts(&config, &opts, None);

        assert!(run_args
            .contains(&"/home/alice/.config/ocx/nix:/home/alice/.config/ocx/nix:rw".to_string()));
    }

    #[test]
    fn test_build_run_opts_user_flake_absent() {
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
        let opencode_config_dir = PathBuf::from("/home/alice/.config/opencode");

        let opts = RunOpts {
            workspace,
            user,
            port: 32768,
            opencode_config_dir,
            host_home_dir: Some(PathBuf::from("/home/alice")),
            user_flake_host_dir: None,
        };

        let run_args = build_run_opts(&config, &opts, None);

        // Ensure no /ocx/nix mount is present
        for arg in &run_args {
            assert!(!arg.contains("/.config/ocx/nix"));
        }
    }

    #[test]
    fn test_build_run_opts_opencode_config_dir_env_set() {
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
        let opencode_config_dir = PathBuf::from("/home/alice/.config/opencode");
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_dir_env = temp_dir.path().to_path_buf();

        let opts = RunOpts {
            workspace,
            user,
            port: 32768,
            opencode_config_dir,
            host_home_dir: Some(PathBuf::from("/home/alice")),
            user_flake_host_dir: None,
        };

        let run_args = build_run_opts(
            &config,
            &opts,
            Some(config_dir_env.to_str().unwrap().to_string()),
        );

        assert!(run_args.contains(&"-v".to_string()));
        assert!(run_args.contains(&format!(
            "{}:/opencode-config-dir:ro",
            config_dir_env.display()
        )));
        assert!(run_args.contains(&"-e".to_string()));
        assert!(run_args.contains(&"OPENCODE_CONFIG_DIR=/opencode-config-dir".to_string()));
    }

    #[test]
    fn test_build_run_opts_opencode_config_dir_env_unset() {
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
        let opencode_config_dir = PathBuf::from("/home/alice/.config/opencode");

        let opts = RunOpts {
            workspace,
            user,
            port: 32768,
            opencode_config_dir,
            host_home_dir: Some(PathBuf::from("/home/alice")),
            user_flake_host_dir: None,
        };

        let run_args = build_run_opts(&config, &opts, None);

        for arg in &run_args {
            assert!(!arg.contains("/opencode-config-dir"));
        }
    }

    #[test]
    fn test_build_run_opts_opencode_config_dir_env_tilde() {
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
        let opencode_config_dir = PathBuf::from("/home/alice/.config/opencode");

        let temp_dir = tempfile::TempDir::new().unwrap();
        let home_dir = temp_dir.path().to_path_buf();
        let config_dir_env = home_dir.join(".config/opencode-custom");
        std::fs::create_dir_all(&config_dir_env).unwrap();

        let opts = RunOpts {
            workspace,
            user,
            port: 32768,
            opencode_config_dir,
            host_home_dir: Some(home_dir),
            user_flake_host_dir: None,
        };

        let run_args = build_run_opts(
            &config,
            &opts,
            Some("~/.config/opencode-custom".to_string()),
        );

        assert!(run_args.contains(&format!(
            "{}:/opencode-config-dir:ro",
            config_dir_env.display()
        )));
    }

    #[test]
    fn test_build_run_opts_workspace_conflict() {
        let config = Config::default();
        let user = ResolvedUser {
            username: "alice".to_string(),
            uid: 1000,
            gid: 1000,
        };
        let workspace_root = PathBuf::from("/home/alice/.config/opencode");
        let workspace = ResolvedWorkspace {
            root: workspace_root.clone(),
            container_path: PathBuf::from("/home/alice/.config/opencode"),
        };
        let opencode_config_dir = workspace_root;

        let opts = RunOpts {
            workspace,
            user,
            port: 32768,
            opencode_config_dir,
            host_home_dir: Some(PathBuf::from("/home/alice")),
            user_flake_host_dir: None,
        };

        let run_args = build_run_opts(&config, &opts, None);

        // Count how many times the config dir is mounted.
        // It should appear exactly ONCE (via the workspace mount), not twice.
        let mount_str = "/home/alice/.config/opencode:/home/alice/.config/opencode:rw";
        let count = run_args.iter().filter(|a| a.as_str() == mount_str).count();

        assert_eq!(
            count, 1,
            "Config dir should be mounted exactly once when it conflicts with workspace root"
        );
    }

    #[test]
    fn test_resolve_config_dir_env_with_tilde() {
        let temp = tempfile::TempDir::new().unwrap();
        let home = temp.path();

        // Create the target dir so .exists() passes
        let target_dir = home.join(".config/my-opencode");
        std::fs::create_dir_all(&target_dir).unwrap();

        let env_val = Some("~/.config/my-opencode".to_string());
        let result = resolve_config_dir_env(env_val, Some(home));

        assert_eq!(result, Some(target_dir));
    }

    #[test]
    fn test_resolve_config_dir_env_absolute() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create the target dir so .exists() passes
        let target_dir = temp.path().join("absolute/path");
        std::fs::create_dir_all(&target_dir).unwrap();

        let env_val = Some(target_dir.to_string_lossy().to_string());
        let result = resolve_config_dir_env(env_val, None);

        assert_eq!(result, Some(target_dir));
    }

    #[test]
    fn test_resolve_config_dir_env_missing() {
        let env_val = Some("/does/not/exist/anywhere/12345".to_string());
        let result = resolve_config_dir_env(env_val, None);

        assert_eq!(result, None);
    }
}
