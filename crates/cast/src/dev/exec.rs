use std::process::ExitStatus;

use anyhow::{bail, Result};
use tracing::{debug, info, info_span};

use crate::config::{ApprovedConfig, Config};
use crate::dev;
use crate::dev::agent::Agent;
use crate::dev::build_command::build_command;
use crate::dev::container_name::resolve_container_name;
use crate::dev::run::{resolve_run_opts, run_in_container, RunOpts, SessionFlags};
use crate::dev::workspace::get_workspace;
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::nix_daemon;
use crate::user::get_user;

/// Build the command vector for `cast exec`.
///
/// When `raw` is true the user-supplied `cmd` is returned as-is (no Nix
/// devshell wrapping).  When `raw` is false, `build_command` wraps `cmd[0]`
/// with the Nix devshell layers exactly as it does for `cast run`.
pub fn build_exec_cmd(config: &Config, opts: &RunOpts, raw: bool, cmd: &[String]) -> Vec<String> {
    if raw || cmd.is_empty() {
        return cmd.to_vec();
    }
    build_command(config, opts, &cmd[0], cmd[1..].to_vec())
}

/// Orchestrate and run a `cast exec` session inside a fresh agent container.
///
/// Unlike `cast shell`, this always starts a **new** container (`docker run
/// --rm`) rather than `docker exec`-ing into an existing one.
///
/// `name_token` is used for container naming and is always `Some(_)` for exec
/// sessions.  It is separate from the TTY mode so that interactive exec (which
/// needs a TTY) can still receive a unique ephemeral container name:
///   - interactive exec → `Some("exec-{invocation_id}")`
///   - headless exec    → `Some("{invocation_id}")`
pub fn exec(
    agent: &dyn Agent,
    config: &ApprovedConfig,
    flags: SessionFlags,
    raw: bool,
    name_token: String,
    cmd: Vec<String>,
) -> Result<ExitStatus> {
    if cmd.is_empty() {
        bail!(
            "cast exec requires a command. \
             Usage: cast exec [FLAGS] <agent> <cmd> [args...]"
        );
    }

    let docker = DockerClient;
    let user = get_user()?;
    let workspace = get_workspace(&user.username)?;

    let port = dev::port::resolve_port(config, agent.name())?;
    let cwd_basename = workspace.root_basename();
    let container_name = resolve_container_name(
        config,
        agent.name(),
        cwd_basename,
        port,
        flags.name.as_deref(),
        Some(&name_token),
    );

    let span = info_span!(
        "exec_session",
        agent = agent.name(),
        container = %container_name,
        port = port,
        raw = raw,
    );
    let _guard = span.enter();

    debug!(port, %container_name, raw, "resolved exec parameters");

    // Always ensure the Nix daemon is running — even --raw mounts /nix.
    nix_daemon::ensure_running(&docker, config)?;

    let version = agent.resolve_version(config)?;
    let image_tag = agent.image_tag(&version);

    info!(
        %image_tag,
        %container_name,
        port,
        raw,
        "starting exec session"
    );

    agent.ensure_image(&docker, config, &user, &version, BuildOptions::default())?;

    let run_opts = resolve_run_opts(user, workspace, port, &flags);
    let exec_cmd = build_exec_cmd(config, &run_opts, raw, &cmd);

    run_in_container(
        &docker,
        agent,
        config,
        &run_opts,
        &container_name,
        &image_tag,
        exec_cmd,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dev::run::TtyMode;
    use crate::dev::workspace::ResolvedWorkspace;
    use crate::user::ResolvedUser;
    use std::path::PathBuf;

    fn alice() -> ResolvedUser {
        ResolvedUser {
            username: "alice".to_string(),
            uid: 1000,
            gid: 1000,
        }
    }

    fn base_opts() -> RunOpts {
        RunOpts {
            workspace: ResolvedWorkspace {
                root: PathBuf::from("/home/alice/project"),
                container_path: PathBuf::from("/home/alice/project"),
            },
            user: alice(),
            port: 8080,
            host_home_dir: Some(PathBuf::from("/home/alice")),
            user_flake_present: false,
            project_flake_present: false,
            tty_mode: TtyMode::Interactive,
            publish: false,
        }
    }

    // ── build_exec_cmd: raw mode ─────────────────────────────────────────────

    #[test]
    fn test_build_exec_cmd_raw_passes_cmd_as_is() {
        let config = Config::default();
        let opts = base_opts();
        let cmd = vec![
            "/bin/bash".to_string(),
            "-c".to_string(),
            "echo hi".to_string(),
        ];
        let result = build_exec_cmd(&config, &opts, true, &cmd);
        assert_eq!(result, cmd, "raw mode must not wrap the command");
    }

    #[test]
    fn test_build_exec_cmd_raw_no_nix_wrap_even_with_flake() {
        let config = Config {
            use_flake: true,
            ..Config::default()
        };
        let opts = RunOpts {
            user_flake_present: true,
            project_flake_present: true,
            ..base_opts()
        };
        let cmd = vec!["/bin/bash".to_string()];
        let result = build_exec_cmd(&config, &opts, true, &cmd);
        // raw=true must bypass Nix wrapping even when flakes are present
        assert_eq!(result, cmd);
        assert!(
            !result.contains(&"nix".to_string()),
            "raw mode must not inject nix develop"
        );
    }

    // ── build_exec_cmd: non-raw mode ─────────────────────────────────────────

    #[test]
    fn test_build_exec_cmd_non_raw_no_flake_is_bare() {
        let config = Config::default(); // use_flake: false, no flake path
        let opts = base_opts(); // user_flake_present: false
        let cmd = vec!["/bin/bash".to_string(), "-c".to_string(), "x".to_string()];
        let result = build_exec_cmd(&config, &opts, false, &cmd);
        // No flakes active → result is the bare command
        assert_eq!(result, cmd);
    }

    #[test]
    fn test_build_exec_cmd_non_raw_wraps_with_nix() {
        let config = Config {
            use_flake: true,
            use_flake_path: None,
            ..Config::default()
        };
        let opts = RunOpts {
            user_flake_present: false,
            project_flake_present: true,
            ..base_opts()
        };
        let cmd = vec!["/bin/bash".to_string()];
        let result = build_exec_cmd(&config, &opts, false, &cmd);
        // With project flake, command is wrapped: nix develop . -c /bin/bash
        assert_eq!(result, vec!["nix", "develop", ".", "-c", "/bin/bash"]);
    }

    #[test]
    fn test_build_exec_cmd_non_raw_splits_cmd_args() {
        // Ensure cmd[1..] is passed as extra_args to build_command.
        let config = Config::default();
        let opts = base_opts();
        let cmd = vec![
            "/bin/bash".to_string(),
            "-c".to_string(),
            "echo hello".to_string(),
        ];
        let result = build_exec_cmd(&config, &opts, false, &cmd);
        // No flakes → bare pass-through; args preserved
        assert_eq!(result, cmd);
    }

    // ── container name token invariants ──────────────────────────────────────

    #[test]
    fn test_exec_interactive_token_contains_exec_prefix() {
        // Interactive exec token is "exec-{invocation_id}"; verify the
        // resulting name contains "exec-".
        use crate::config::Config;
        use crate::dev::container_name::resolve_container_name;

        let cfg = Config::default();
        let token = format!("exec-{}", "abc123");
        let name = resolve_container_name(&cfg, "opencode", "my-app", 8080, None, Some(&token));
        assert!(
            name.contains("exec-"),
            "interactive exec container name should contain 'exec-': {}",
            name
        );
    }

    #[test]
    fn test_exec_headless_token_no_exec_prefix() {
        // Headless exec token is the bare invocation_id (no "exec-" prefix).
        use crate::config::Config;
        use crate::dev::container_name::resolve_container_name;

        let cfg = Config::default();
        let token = "abc123"; // bare invocation_id
        let name = resolve_container_name(&cfg, "opencode", "my-app", 8080, None, Some(token));
        // Name ends with the raw token, not exec-<token>
        assert!(
            name.ends_with(token),
            "headless exec name should end with bare token: {}",
            name
        );
        assert!(
            !name.contains("exec-"),
            "headless exec name should NOT contain 'exec-': {}",
            name
        );
    }
}
