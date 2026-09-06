use std::path::Path;
use std::process::ExitStatus;

use crate::config::{ApprovedConfig, Config};
use crate::dev::build_command::build_command;
use crate::dev::service::ServiceStatus;
use crate::dev::service_context::ServiceContext;
use crate::docker::client::DockerClient;
use crate::nix_daemon;
use crate::user::get_user;
use anyhow::{Result, bail};

/// Build the command vector for `cast exec`.
///
/// When `raw` is true the user-supplied `cmd` is returned as-is (no Nix
/// devshell wrapping).  When `raw` is false, `build_command` wraps `cmd[0]`
/// with the Nix devshell layers exactly as it does for `cast run`.
pub fn build_exec_cmd(
    config: &Config,
    container_username: &str,
    raw: bool,
    cmd: &[String],
) -> Vec<String> {
    if raw || cmd.is_empty() {
        return cmd.to_vec();
    }
    build_command(config, container_username, &cmd[0], cmd[1..].to_vec())
}

/// Options for a command targeting a worktree service.
pub struct ServiceExecOptions<'a> {
    pub service_name: Option<&'a str>,
    pub container_username: &'a str,
    pub container_workdir: &'a Path,
    pub headless: bool,
    pub raw: bool,
}

/// Build `docker exec` arguments for a command targeting a worktree service.
pub fn build_service_exec_args(
    config: &Config,
    context: &ServiceContext,
    options: &ServiceExecOptions<'_>,
    cmd: &[String],
) -> Vec<String> {
    let mut args = vec!["exec".to_string()];
    if !options.headless {
        args.push("-it".to_string());
    }
    args.extend([
        "--workdir".to_string(),
        options.container_workdir.to_string_lossy().into_owned(),
        context.container_name(options.service_name),
    ]);
    args.extend(build_exec_cmd(
        config,
        options.container_username,
        options.raw,
        cmd,
    ));
    args
}

fn validate_service_exec_status(status: ServiceStatus, container_name: &str) -> Result<()> {
    if status == ServiceStatus::Running {
        return Ok(());
    }

    bail!("service is {status}: {container_name}; run `cast up` first")
}

/// Execute a command in the selected worktree service.
pub fn exec(
    config: &ApprovedConfig,
    context: &ServiceContext,
    service_name: Option<&str>,
    headless: bool,
    raw: bool,
    cmd: Vec<String>,
) -> Result<ExitStatus> {
    if cmd.is_empty() {
        bail!("cast exec requires a command")
    }

    let container_name = context.container_name(service_name);
    validate_service_exec_status(
        crate::dev::service::status(context, service_name)?,
        &container_name,
    )?;

    let docker = DockerClient;
    nix_daemon::ensure_running(&docker, config)?;

    let user = get_user()?;
    let workspace = context.workspace(dirs::home_dir().as_deref(), &user.username);
    let container_workdir = context.container_workdir(&workspace);
    let args = build_service_exec_args(
        config,
        context,
        &ServiceExecOptions {
            service_name,
            container_username: &user.username,
            container_workdir: &container_workdir,
            headless,
            raw,
        },
        &cmd,
    );

    docker.interactive_command(args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dev::service_context::ServiceContext;
    use std::path::{Path, PathBuf};

    #[test]
    fn service_exec_enters_configured_shells_in_the_invocation_directory() {
        let config = Config {
            sandbox_shell: Some("~/.config/cast/nix#default".to_string()),
            project_shell: Some(".#ai".to_string()),
            ..Config::default()
        };
        let context = ServiceContext {
            worktree_root: PathBuf::from("/home/alice/projects/my-app"),
            git_common_dir: PathBuf::from("/home/alice/projects/my-app/.git"),
            relative_cwd: PathBuf::from("crates/app"),
            workspace_id: "a1b2c3d4e5f6".to_string(),
        };
        let command = vec!["cargo".to_string(), "test".to_string()];

        let args = build_service_exec_args(
            &config,
            &context,
            &ServiceExecOptions {
                service_name: None,
                container_username: "alice",
                container_workdir: Path::new("/home/alice/projects/my-app/crates/app"),
                headless: false,
                raw: false,
            },
            &command,
        );

        assert_eq!(
            args,
            vec![
                "exec",
                "-it",
                "--workdir",
                "/home/alice/projects/my-app/crates/app",
                "cast-my-app-a1b2c3d4e5f6",
                "nix",
                "develop",
                "/home/alice/.config/cast/nix#default",
                "-c",
                "nix",
                "develop",
                ".#ai",
                "-c",
                "cargo",
                "test",
            ]
        );
    }

    fn service_context_fixture() -> ServiceContext {
        ServiceContext {
            worktree_root: PathBuf::from("/home/alice/projects/my-app"),
            git_common_dir: PathBuf::from("/home/alice/projects/my-app/.git"),
            relative_cwd: PathBuf::new(),
            workspace_id: "a1b2c3d4e5f6".to_string(),
        }
    }

    fn wrapped_config() -> Config {
        Config {
            sandbox_shell: Some("~/.config/cast/nix#default".to_string()),
            project_shell: Some(".#ai".to_string()),
            ..Config::default()
        }
    }

    #[test]
    fn raw_service_exec_skips_devshell_wrapping() {
        let args = build_service_exec_args(
            &wrapped_config(),
            &service_context_fixture(),
            &ServiceExecOptions {
                service_name: None,
                container_username: "alice",
                container_workdir: Path::new("/home/alice/projects/my-app"),
                headless: false,
                raw: true,
            },
            &["cargo".to_string(), "test".to_string()],
        );

        assert_eq!(
            args,
            vec![
                "exec",
                "-it",
                "--workdir",
                "/home/alice/projects/my-app",
                "cast-my-app-a1b2c3d4e5f6",
                "cargo",
                "test",
            ]
        );
    }

    #[test]
    fn headless_service_exec_allocates_no_terminal() {
        let args = build_service_exec_args(
            &Config::default(),
            &service_context_fixture(),
            &ServiceExecOptions {
                service_name: None,
                container_username: "alice",
                container_workdir: Path::new("/home/alice/projects/my-app"),
                headless: true,
                raw: false,
            },
            &["echo".to_string(), "hi".to_string()],
        );

        assert_eq!(
            args,
            vec![
                "exec",
                "--workdir",
                "/home/alice/projects/my-app",
                "cast-my-app-a1b2c3d4e5f6",
                "echo",
                "hi",
            ]
        );
    }

    #[test]
    fn service_exec_routes_to_a_named_service() {
        let args = build_service_exec_args(
            &Config::default(),
            &service_context_fixture(),
            &ServiceExecOptions {
                service_name: Some("review"),
                container_username: "alice",
                container_workdir: Path::new("/home/alice/projects/my-app"),
                headless: true,
                raw: true,
            },
            &["echo".to_string()],
        );

        assert!(
            args.contains(&"cast-my-app-a1b2c3d4e5f6-review".to_string()),
            "named exec must target the named service container: {args:?}"
        );
    }

    #[test]
    fn service_exec_rejects_a_stopped_service() {
        let error =
            validate_service_exec_status(ServiceStatus::Stopped, "cast-my-app-a1b2c3d4e5f6")
                .expect_err("a stopped service must reject exec");

        assert_eq!(
            error.to_string(),
            "service is stopped: cast-my-app-a1b2c3d4e5f6; run `cast up` first"
        );
    }

    #[test]
    fn service_exec_rejects_an_absent_service() {
        let error = validate_service_exec_status(
            crate::dev::service::ServiceStatus::Absent,
            "cast-my-app-a1b2c3d4e5f6",
        )
        .expect_err("an absent service must reject exec");

        assert_eq!(
            error.to_string(),
            "service is absent: cast-my-app-a1b2c3d4e5f6; run `cast up` first"
        );
    }

    // ── build_exec_cmd: raw mode ─────────────────────────────────────────────

    #[test]
    fn test_build_exec_cmd_raw_passes_cmd_as_is() {
        let config = Config::default();
        let cmd = vec![
            "/bin/bash".to_string(),
            "-c".to_string(),
            "echo hi".to_string(),
        ];
        let result = build_exec_cmd(&config, "alice", true, &cmd);
        assert_eq!(result, cmd, "raw mode must not wrap the command");
    }

    #[test]
    fn test_build_exec_cmd_raw_no_nix_wrap_even_with_shell_refs() {
        let config = Config {
            sandbox_shell: Some("~/.config/cast/nix#default".to_string()),
            project_shell: Some(".#ai".to_string()),
            ..Config::default()
        };
        let cmd = vec!["/bin/bash".to_string()];
        let result = build_exec_cmd(&config, "alice", true, &cmd);
        // raw=true must bypass Nix wrapping even when both layers are active
        assert_eq!(result, cmd);
        assert!(
            !result.contains(&"nix".to_string()),
            "raw mode must not inject nix develop"
        );
    }

    // ── build_exec_cmd: non-raw mode ─────────────────────────────────────────

    #[test]
    fn test_build_exec_cmd_non_raw_no_shell_ref_is_bare() {
        let config = Config::default();
        let cmd = vec!["/bin/bash".to_string(), "-c".to_string(), "x".to_string()];
        let result = build_exec_cmd(&config, "alice", false, &cmd);
        // No layers active → result is the bare command
        assert_eq!(result, cmd);
    }

    #[test]
    fn test_build_exec_cmd_non_raw_wraps_with_project_ref() {
        let config = Config {
            project_shell: Some(".#ai".to_string()),
            ..Config::default()
        };
        let cmd = vec!["/bin/bash".to_string()];
        let result = build_exec_cmd(&config, "alice", false, &cmd);
        assert_eq!(result, vec!["nix", "develop", ".#ai", "-c", "/bin/bash"]);
    }

    #[test]
    fn test_build_exec_cmd_non_raw_splits_cmd_args_with_flake() {
        // With a project layer active, cmd[0] is used as the binary and
        // cmd[1..] is forwarded as extra args to build_command.
        let config = Config {
            project_shell: Some(".#ai".to_string()),
            ..Config::default()
        };
        let cmd = vec![
            "/bin/bash".to_string(),
            "-c".to_string(),
            "echo hello".to_string(),
        ];
        let result = build_exec_cmd(&config, "alice", false, &cmd);
        assert_eq!(
            result,
            vec![
                "nix",
                "develop",
                ".#ai",
                "-c",
                "/bin/bash",
                "-c",
                "echo hello"
            ]
        );
    }

    // ── empty cmd handling ────────────────────────────────────────────────────

    #[test]
    fn test_build_exec_cmd_empty_returns_empty() {
        // build_exec_cmd with empty cmd returns empty regardless of raw flag.
        // The caller (exec()) bails before build_exec_cmd is reached, but
        // the function itself must not panic on empty input.
        let config = Config::default();
        let empty: Vec<String> = vec![];
        assert_eq!(build_exec_cmd(&config, "alice", false, &empty), empty);
        assert_eq!(build_exec_cmd(&config, "alice", true, &empty), empty);
    }

    #[test]
    fn test_exec_empty_cmd_returns_error() {
        use crate::config::ApprovedConfig;

        let config = ApprovedConfig::assume_approved_for_test(Config::default());
        let context = ServiceContext {
            worktree_root: PathBuf::from("/home/alice/projects/my-app"),
            git_common_dir: PathBuf::from("/home/alice/projects/my-app/.git"),
            relative_cwd: PathBuf::new(),
            workspace_id: "a1b2c3d4e5f6".to_string(),
        };
        let result = exec(&config, &context, None, false, false, vec![]);
        assert!(result.is_err(), "exec with empty cmd must return an error");
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("requires a command"),
            "error message should mention the missing command, got: {}",
            msg
        );
    }
}
