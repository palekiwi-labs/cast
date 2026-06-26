use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use tracing::{debug, info, info_span};

use crate::config::{ApprovedConfig, Config};
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

/// Whether the session uses a pseudo-TTY (interactive) or not (headless).
#[derive(Debug, Clone, PartialEq)]
pub enum TtyMode {
    Interactive,
    Headless,
}

/// The execution mode for a `cast run` session.
///
/// `Headless` carries the uniqueness token used for the ephemeral container
/// name, making it impossible for callers to set `headless` without also
/// providing the token.
#[derive(Debug, Clone)]
pub enum RunMode {
    Interactive,
    Headless { token: String },
}

impl From<&RunMode> for TtyMode {
    fn from(mode: &RunMode) -> Self {
        match mode {
            RunMode::Interactive => TtyMode::Interactive,
            RunMode::Headless { .. } => TtyMode::Headless,
        }
    }
}

/// Flags controlling the session execution mode.
#[derive(Debug)]
pub struct SessionFlags {
    pub mode: RunMode,
    pub name: Option<String>,
}

/// Generic options for building the Docker run command.
/// Contains only agent-agnostic data; each agent resolves its own
/// additional context inside `Agent::extra_run_args`.
pub struct RunOpts {
    pub workspace: ResolvedWorkspace,
    pub user: ResolvedUser,
    pub port: u16,
    pub host_home_dir: Option<PathBuf>,
    pub user_flake_present: bool,
    pub project_flake_present: bool,
    pub tty_mode: TtyMode,
    pub publish: bool,
}

use std::process::ExitStatus;

/// Orchestrate and run an agent session inside the dev container.
pub fn run_agent(
    agent: &dyn Agent,
    config: &ApprovedConfig,
    flags: SessionFlags,
    extra_args: Vec<String>,
) -> Result<ExitStatus> {
    let start_time = Instant::now();
    let docker = DockerClient;
    let user = get_user()?;
    let workspace = get_workspace(&user.username)?;

    // Resolve port and container name early for span.
    let port = dev::port::resolve_port(config, agent.name())?;
    let cwd_basename = workspace.root_basename();
    let token = match &flags.mode {
        RunMode::Headless { token } => Some(token.as_str()),
        RunMode::Interactive => None,
    };
    let container_name = resolve_container_name(
        config,
        agent.name(),
        cwd_basename,
        port,
        flags.name.as_deref(),
        token,
    );

    let span = info_span!(
        "agent_session",
        agent = agent.name(),
        container = %container_name,
        port = port
    );
    let _guard = span.enter();

    debug!(port, %container_name, "resolved session parameters");

    // Ensure the Nix daemon is running.
    nix_daemon::ensure_running(&docker, config)?;

    // Resolve the version and image for this agent, and ensure it exists locally.
    let version = agent.resolve_version(config)?;
    let image_tag = agent.image_tag(&version);

    info!(
        %image_tag,
        %container_name,
        port,
        "starting agent session"
    );

    agent.ensure_image(&docker, config, &user, &version, BuildOptions::default())?;

    let env: HashMap<String, String> = std::env::vars().collect();
    let run_opts = resolve_run_opts(user, workspace, port, &flags);

    // Prepare host-side side effects before building arguments.
    agent.prepare_host(config, &run_opts)?;

    // Build generic docker run flags, then append agent-specific ones.
    let mut opts = build_docker_run_flags(config, &run_opts);
    opts.extend(agent.extra_run_args(config, &run_opts, &env)?);

    // Announce nix devshell layers before handing off to docker, so the
    // user knows what environment is being loaded.  The global flake is
    // the outermost layer and is announced here; the project flake
    // announces itself via its own shellHook (echo ... >&2).
    if run_opts.user_flake_present {
        info!("loading global nix devshell");
        eprintln!("Loading global nix devshell...");
    }

    // Build the full command and exec into the container.
    let cmd = agent.build_command(config, &run_opts, extra_args);
    let docker_args = build_run_args(&container_name, &image_tag, opts, Some(cmd));

    let status = match run_opts.tty_mode {
        TtyMode::Interactive => docker.interactive_command(docker_args)?,
        TtyMode::Headless => docker.headless_command(docker_args, &container_name)?,
    };

    let duration = start_time.elapsed();
    info!(
        exit_code = status.code(),
        duration_secs = duration.as_secs(),
        "agent session ended"
    );

    Ok(status)
}

/// Resolve the generic options for a session, detecting flake presence.
pub fn resolve_run_opts(
    user: ResolvedUser,
    workspace: ResolvedWorkspace,
    port: u16,
    flags: &SessionFlags,
) -> RunOpts {
    let host_home_dir = dirs::home_dir();
    let user_flake_present = host_home_dir
        .as_ref()
        .filter(|h| h.join(".config/cast/nix/flake.nix").exists())
        .is_some();

    let project_flake_present = workspace.root.join("flake.nix").exists();

    RunOpts {
        workspace,
        user,
        port,
        host_home_dir,
        user_flake_present,
        project_flake_present,
        tty_mode: TtyMode::from(&flags.mode),
        publish: matches!(flags.mode, RunMode::Interactive),
    }
}

/// Build the generic set of Docker run flags that apply to every agent.
///
/// Agent-specific arguments (env vars, program-specific mounts, etc.) are
/// NOT included here — each agent appends them via `Agent::extra_run_args`.
pub fn build_docker_run_flags(config: &Config, opts: &RunOpts) -> Vec<String> {
    // TTY flags: interactive gets "-it", headless gets neither.
    // Headless runs are fire-and-forget; the agent receives its input via
    // extra_args, not stdin. Passing -i with an inherited terminal stdin
    // causes docker to block indefinitely waiting for EOF.
    let tty_flags: Vec<String> = match opts.tty_mode {
        TtyMode::Interactive => vec!["-it".to_string()],
        TtyMode::Headless => vec![],
    };

    let mut run_args: Vec<String> = vec!["--rm".to_string()];
    run_args.extend(tty_flags);
    run_args.extend([
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
    ]);

    // Port publishing: only when both config and opts enable it.
    if config.publish_port && opts.publish {
        run_args.push("-p".to_string());
        run_args.push(format!("{}:80", opts.port));
    }

    // Env files.
    run_args.extend(build_env_file_args(
        &opts.workspace.root,
        opts.host_home_dir.as_deref(),
    ));

    // Environment: user identity (unconditional in both modes).
    run_args.extend(["-e".to_string(), format!("USER={}", opts.user.username)]);

    // Environment: terminal capabilities (interactive only) or NO_COLOR (headless).
    match opts.tty_mode {
        TtyMode::Interactive => {
            run_args.extend([
                "-e".to_string(),
                "TERM=xterm-256color".to_string(),
                "-e".to_string(),
                "COLORTERM=truecolor".to_string(),
                "-e".to_string(),
                "FORCE_COLOR=1".to_string(),
            ]);
        }
        TtyMode::Headless => {
            run_args.extend(["-e".to_string(), "NO_COLOR=1".to_string()]);
        }
    }

    // MCP server URL injection.
    let mcp_url = format!("http://host.docker.internal:{}/mcp", config.mcp.port);
    run_args.extend(["-e".to_string(), format!("CAST_MCP_URL={}", mcp_url)]);

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

    // ── Phase 1: RunMode → TtyMode conversion ───────────────────────────────

    #[test]
    fn test_run_mode_interactive_maps_to_interactive_tty() {
        assert_eq!(TtyMode::from(&RunMode::Interactive), TtyMode::Interactive);
    }

    #[test]
    fn test_run_mode_headless_maps_to_headless_tty() {
        let mode = RunMode::Headless {
            token: "tok".to_string(),
        };
        assert_eq!(TtyMode::from(&mode), TtyMode::Headless);
    }

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn make_interactive_opts(
        user: ResolvedUser,
        workspace: ResolvedWorkspace,
        port: u16,
    ) -> RunOpts {
        RunOpts {
            workspace,
            user,
            port,
            host_home_dir: Some(PathBuf::from("/home/alice")),
            user_flake_present: false,
            project_flake_present: false,
            tty_mode: TtyMode::Interactive,
            publish: true,
        }
    }

    fn make_headless_opts(user: ResolvedUser, workspace: ResolvedWorkspace, port: u16) -> RunOpts {
        RunOpts {
            workspace,
            user,
            port,
            host_home_dir: Some(PathBuf::from("/home/alice")),
            user_flake_present: false,
            project_flake_present: false,
            tty_mode: TtyMode::Headless,
            publish: false,
        }
    }

    fn alice_user() -> ResolvedUser {
        ResolvedUser {
            username: "alice".to_string(),
            uid: 1000,
            gid: 1000,
        }
    }

    fn alice_workspace() -> ResolvedWorkspace {
        ResolvedWorkspace {
            root: PathBuf::from("/home/alice/project"),
            container_path: PathBuf::from("/home/alice/project"),
        }
    }

    // ── Phase 2: build_docker_run_flags — interactive mode ──────────────────

    #[test]
    fn test_build_docker_run_flags_basic() {
        let config = Config::default();
        let opts = make_interactive_opts(alice_user(), alice_workspace(), 32768);

        let run_args = build_docker_run_flags(&config, &opts);

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

        // MCP URL injection
        assert!(run_args.contains(&"CAST_MCP_URL=http://host.docker.internal:8080/mcp".to_string()));
    }

    #[test]
    fn test_build_docker_run_flags_mcp_custom_port() {
        let mut config = Config::default();
        config.mcp.port = 9000;
        let opts = make_interactive_opts(alice_user(), alice_workspace(), 32768);

        let run_args = build_docker_run_flags(&config, &opts);
        assert!(run_args.contains(&"CAST_MCP_URL=http://host.docker.internal:9000/mcp".to_string()));
    }

    #[test]
    fn test_build_docker_run_flags_add_host_enabled() {
        let config = Config {
            add_host_docker_internal: true,
            ..Config::default()
        };
        let opts = make_interactive_opts(alice_user(), alice_workspace(), 32768);

        let run_args = build_docker_run_flags(&config, &opts);

        let add_host_pos = run_args.iter().position(|r| r == "--add-host");
        assert!(add_host_pos.is_some(), "Should contain --add-host");
        assert_eq!(
            run_args[add_host_pos.unwrap() + 1],
            "host.docker.internal:host-gateway"
        );
    }

    #[test]
    fn test_build_docker_run_flags_add_host_disabled() {
        let config = Config {
            add_host_docker_internal: false,
            ..Config::default()
        };
        let opts = make_interactive_opts(alice_user(), alice_workspace(), 32768);

        let run_args = build_docker_run_flags(&config, &opts);

        assert!(
            !run_args.contains(&"--add-host".to_string()),
            "Should NOT contain --add-host when disabled"
        );
    }

    #[test]
    fn test_build_docker_run_flags_shadow_mounts() {
        let config = Config {
            forbidden_paths: vec!["secrets".to_string()],
            ..Config::default()
        };

        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        std::fs::create_dir(root.join("secrets")).unwrap();

        let workspace = ResolvedWorkspace {
            root,
            container_path: PathBuf::from("/home/alice/project"),
        };

        let opts = make_interactive_opts(alice_user(), workspace, 32768);
        let run_args = build_docker_run_flags(&config, &opts);

        assert!(run_args.contains(
            &"/home/alice/project/secrets:ro,noexec,nosuid,size=1k,mode=000".to_string()
        ));
    }

    #[test]
    fn test_resolve_run_opts_detects_flakes() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        // Create a flake.nix in the workspace
        std::fs::write(root.join("flake.nix"), "").unwrap();

        let workspace = ResolvedWorkspace {
            root: root.clone(),
            container_path: PathBuf::from("/work"),
        };

        let flags = SessionFlags {
            mode: RunMode::Interactive,
            name: None,
        };
        let opts = resolve_run_opts(alice_user(), workspace, 8080, &flags);

        assert!(opts.project_flake_present);
        // user_flake_present depends on host home dir, which we can't easily mock here
        // but we verified the logic is correct.
    }

    // ── Phase 2: build_docker_run_flags — headless mode ─────────────────────

    #[test]
    fn test_build_docker_run_flags_headless_no_tty_flags() {
        let config = Config::default();
        let opts = make_headless_opts(alice_user(), alice_workspace(), 32768);
        let run_args = build_docker_run_flags(&config, &opts);

        assert!(
            !run_args.contains(&"-i".to_string()),
            "Should NOT contain -i in headless mode"
        );
        assert!(
            !run_args.contains(&"-it".to_string()),
            "Should NOT contain -it in headless mode"
        );
        assert!(
            !run_args.contains(&"-t".to_string()),
            "Should NOT contain -t in headless mode"
        );
    }

    #[test]
    fn test_build_docker_run_flags_headless_no_port() {
        let config = Config {
            publish_port: true,
            ..Config::default()
        };
        let opts = make_headless_opts(alice_user(), alice_workspace(), 32768);
        let run_args = build_docker_run_flags(&config, &opts);

        assert!(
            !run_args.contains(&"-p".to_string()),
            "Should NOT publish port in headless mode even if config.publish_port is true"
        );
    }

    #[test]
    fn test_build_docker_run_flags_headless_no_color_vars() {
        let config = Config::default();
        let opts = make_headless_opts(alice_user(), alice_workspace(), 32768);
        let run_args = build_docker_run_flags(&config, &opts);

        assert!(
            !run_args.iter().any(|a| a.contains("FORCE_COLOR")),
            "Should NOT inject FORCE_COLOR in headless mode"
        );
        assert!(
            !run_args.iter().any(|a| a.contains("COLORTERM")),
            "Should NOT inject COLORTERM in headless mode"
        );
        assert!(
            !run_args.iter().any(|a| a.contains("xterm-256color")),
            "Should NOT inject TERM=xterm-256color in headless mode"
        );
    }

    #[test]
    fn test_build_docker_run_flags_headless_injects_no_color() {
        let config = Config::default();
        let opts = make_headless_opts(alice_user(), alice_workspace(), 32768);
        let run_args = build_docker_run_flags(&config, &opts);

        assert!(
            run_args.contains(&"NO_COLOR=1".to_string()),
            "Should inject NO_COLOR=1 in headless mode"
        );
    }

    #[test]
    fn test_build_docker_run_flags_headless_user_present() {
        let config = Config::default();
        let opts = make_headless_opts(alice_user(), alice_workspace(), 32768);
        let run_args = build_docker_run_flags(&config, &opts);

        assert!(
            run_args.contains(&"USER=alice".to_string()),
            "USER= must be present in headless mode"
        );
    }

    #[test]
    fn test_build_docker_run_flags_interactive_port_published() {
        let config = Config {
            publish_port: true,
            ..Config::default()
        };
        let opts = make_interactive_opts(alice_user(), alice_workspace(), 32768);
        let run_args = build_docker_run_flags(&config, &opts);

        assert!(
            run_args.contains(&"-p".to_string()),
            "Should publish port in interactive mode when config.publish_port is true"
        );
    }
}
