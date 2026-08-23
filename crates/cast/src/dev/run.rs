use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use tracing::{debug, info, info_span};

use crate::config::{ApprovedConfig, Config};
use crate::dev;
use crate::dev::agent::Agent;
use crate::dev::container_name::resolve_container_name;
use crate::dev::env_file::{build_env_file_args, build_env_passthrough_args, reserved_names_in};
use crate::dev::shadow_mounts::{build_shadow_mount_args, resolve_shadow_mounts};
use crate::dev::universal::registry::all_agents;
use crate::dev::universal::volumes::build_universal_run_args;
use crate::dev::volumes::build_extra_volume_args;
use crate::dev::workspace::{ResolvedWorkspace, get_workspace};
use crate::docker::BuildOptions;
use crate::docker::args::build_run_args;
use crate::docker::client::DockerClient;
use crate::nix_daemon;
use crate::user::{ResolvedUser, get_user};

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
    /// Whether to publish the agent's port to the host. `false` means no publish.
    pub publish: bool,
}

/// Generic options for building the Docker run command.
/// Contains only agent-agnostic data; each agent contributes its own
/// context via `Agent::config_mount_args`.
pub struct RunOpts {
    pub workspace: ResolvedWorkspace,
    pub user: ResolvedUser,
    pub port: u16,
    pub host_home_dir: Option<PathBuf>,
    /// The host machine's name, injected as `CAST_HOST_NAME` into the
    /// container for diagnostics grouping. Always non-empty (falls back to
    /// `"unknown"` when the hostname cannot be resolved).
    pub host_name: String,
    pub user_flake_present: bool,
    pub project_flake_present: bool,
    pub tty_mode: TtyMode,
    /// Whether to publish the container's port to the host. `false` means no publish.
    pub publish: bool,
}

use std::process::ExitStatus;

/// Dispatch the final `docker run` command, logging session duration.
///
/// Combines the generic `docker_flags` with agent-specific `extra_args`,
/// builds the full argument vector, and dispatches on `tty_mode`. This is
/// the shared tail used by [`run_in_container`], which serves both
/// `cast run` (via [`run_agent`]) and `cast exec`.
fn dispatch_run(
    docker: &DockerClient,
    run_opts: &RunOpts,
    container_name: &str,
    image_tag: &str,
    docker_flags: Vec<String>,
    extra_args: Vec<String>,
    cmd: Vec<String>,
) -> Result<ExitStatus> {
    let start_time = Instant::now();

    let mut all_flags = docker_flags;
    all_flags.extend(extra_args);

    let docker_args = build_run_args(container_name, image_tag, all_flags, Some(cmd));

    let status = match run_opts.tty_mode {
        TtyMode::Interactive => docker.interactive_command(docker_args)?,
        TtyMode::Headless => docker.headless_command(docker_args, container_name)?,
    };

    let duration = start_time.elapsed();
    info!(
        exit_code = status.code(),
        duration_secs = duration.as_secs(),
        "session ended"
    );

    Ok(status)
}

/// Assemble the agent-specific `docker run` args for a session.
///
/// Both `cast run` and `cast exec` route through this so the two commands
/// share an identical mount topology: the shared `{ns}-cache`/`{ns}-local`
/// data volumes, the cross-harness `~/.agents` bind mount, and the union of
/// every agent's config-directory mounts (universal mounts are the
/// unconditional default).
fn build_session_run_args(config: &Config, run_opts: &RunOpts) -> Result<Vec<String>> {
    build_universal_run_args(all_agents(), config, run_opts)
}

/// Shared container-run core used by both `run_agent` and `exec`.
///
/// Takes a pre-resolved `RunOpts`, container name, image tag, and final
/// command vector. Handles: per-agent `prepare_host` for every known agent,
/// `universal::prepare_host` (creates the cross-harness `~/.agents` dir),
/// docker flags, the universal agent args, dispatch on tty_mode, and duration
/// logging.
pub fn run_in_container(
    docker: &DockerClient,
    config: &ApprovedConfig,
    run_opts: &RunOpts,
    container_name: &str,
    image_tag: &str,
    cmd: Vec<String>,
) -> Result<ExitStatus> {
    // Names only, and set-but-empty host vars are excluded: `-e NAME` beats
    // --env-file, so an empty host value would silently shadow a real one
    // from cast.env.
    let host_env_names: BTreeSet<String> = std::env::vars()
        .filter(|(_, value)| !value.is_empty())
        .map(|(name, _)| name)
        .collect();

    // warn! writes to the file log; eprintln! writes to the console. The
    // reserved-name drop would otherwise be invisible: the run succeeds
    // and the variable is simply absent inside the container.
    let reserved = reserved_names_in(&effective_env_passthrough(config));
    if !reserved.is_empty() {
        eprintln!(
            "Warning: env_passthrough/extra_env_passthrough lists reserved name(s) [{}]: the sandbox owns them, \
             so they are never forwarded. Remove them from env_passthrough or extra_env_passthrough.",
            reserved
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // Prepare host directories for ALL known agents so every config dir and
    // cache dir exists before the container starts (universal mounts).
    for ag in all_agents() {
        ag.prepare_host(config, run_opts)?;
    }

    // Prepare cross-harness host directories (e.g. ~/.agents) shared by every
    // session regardless of which agents are included.
    dev::universal::prepare_host(run_opts)?;

    let docker_flags = build_docker_run_flags(config, run_opts, &host_env_names);
    let extra_args = build_session_run_args(config, run_opts)?;

    dispatch_run(
        docker,
        run_opts,
        container_name,
        image_tag,
        docker_flags,
        extra_args,
        cmd,
    )
}

/// Orchestrate and run an agent session inside the dev container.
pub fn run_agent(
    agent: &dyn Agent,
    config: &ApprovedConfig,
    flags: SessionFlags,
    extra_args: Vec<String>,
) -> Result<ExitStatus> {
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

    // Resolve the single shared dev image and ensure it exists locally.
    let image_tag = dev::image::image_tag();

    info!(
        %image_tag,
        %container_name,
        port,
        "starting agent session"
    );

    dev::image::ensure_dev_image(&docker, config, &user, BuildOptions::default())?;

    // Scaffold the global flake (if absent) before detecting its presence so
    // the first run on a clean host works with no manual setup.
    scaffold_global_flake();
    // Seed the global cast.json (if absent) so the numtide cache reaches the
    // daemon server-side on the first run.
    scaffold_global_cast_json();

    let run_opts = resolve_run_opts(user, workspace, port, &flags);

    // Announce nix devshell layers before handing off to docker, so the
    // user knows what environment is being loaded.  The global flake is
    // the outermost layer and is announced here; the project flake
    // announces itself via its own shellHook (echo ... >&2).
    if run_opts.user_flake_present {
        info!("loading global nix devshell");
        eprintln!("Loading global nix devshell...");
    }

    let cmd = agent.build_command(config, &run_opts, extra_args);

    run_in_container(&docker, config, &run_opts, &container_name, &image_tag, cmd)
}

/// Auto-scaffold the global cast flake on a new host so the first
/// `cast run`/`cast exec` works with no manual setup. A loud notice is emitted
/// so the user knows a file was created on their behalf.
///
/// This is a side-effecting, host-mutating step and is deliberately kept OUT
/// of [`resolve_run_opts`] (which is pure detection) so unit tests never write
/// to the real `$HOME`. Callers must invoke this before `resolve_run_opts` so
/// flake detection observes the scaffolded file on the first run.
pub fn scaffold_global_flake() {
    let Some(home) = dirs::home_dir() else {
        return;
    };
    match crate::dev::global_flake::scaffold_if_missing(&home) {
        Ok(true) => {
            info!("scaffolded global nix flake");
            eprintln!(
                "Created global nix flake at {}",
                home.join(".config/cast/nix/flake.nix").display()
            );
        }
        Ok(false) => {}
        Err(e) => {
            tracing::warn!(error = %e, "failed to scaffold global nix flake");
        }
    }
}

/// Auto-scaffold the global cast config (`cast.json`) on a new host so the
/// first `cast run`/`cast exec` provisions the numtide binary cache
/// daemon-side, avoiding a silent regression to multi-minute source builds.
///
/// Like [`scaffold_global_flake`] this is a side-effecting, host-mutating step
/// kept OUT of [`resolve_run_opts`] so unit tests never write to the real
/// `$HOME`.
pub fn scaffold_global_cast_json() {
    let Some(home) = dirs::home_dir() else {
        return;
    };
    match crate::dev::global_config::scaffold_if_missing(&home) {
        Ok(true) => {
            info!("scaffolded global cast config");
            eprintln!(
                "Created global cast config at {}",
                home.join(".config/cast/cast.json").display()
            );
        }
        Ok(false) => {}
        Err(e) => {
            tracing::warn!(error = %e, "failed to scaffold global cast config");
        }
    }
}

/// Resolve the generic options for a session, detecting flake presence.
///
/// Pure detection only: this reads the host home and workspace to determine
/// flake presence and host identity. It performs no host mutation — global
/// flake scaffolding is the caller's responsibility via [`scaffold_global_flake`].
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

    // Soft-fail: the hostname is diagnostic metadata, not load-bearing for
    // the sandbox. A failed lookup degrades to "unknown" + a warning rather
    // than aborting a working session.
    let host_name = crate::host::get_hostname().unwrap_or_else(|e| {
        tracing::warn!(error = %e, "failed to resolve host name; using fallback");
        "unknown".to_string()
    });

    RunOpts {
        workspace,
        user,
        port,
        host_home_dir,
        host_name,
        user_flake_present,
        project_flake_present,
        tty_mode: TtyMode::from(&flags.mode),
        publish: flags.publish,
    }
}

/// Effective env passthrough allowlist: `env_passthrough` (base) then
/// `extra_env_passthrough` (per-project additions), before the single
/// filter pass (validity, reserved names, unset/empty, dedupe, sort)
/// applied inside [`build_env_passthrough_args`]. The concat lives here
/// so every consumer — the reserved-name warning and the flags builder —
/// sees the same list and cannot drift apart.
fn effective_env_passthrough(config: &Config) -> Vec<String> {
    let mut names = config.env_passthrough.clone();
    names.extend(config.extra_env_passthrough.iter().cloned());
    names
}

/// Build the generic set of Docker run flags that apply to every agent.
///
/// Agent-specific arguments (program-specific mounts, etc.) are NOT included
/// here — each agent contributes them via `Agent::config_mount_args`.
pub fn build_docker_run_flags(
    config: &Config,
    opts: &RunOpts,
    host_env_names: &BTreeSet<String>,
) -> Vec<String> {
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

    // Port publishing: opt-in via --publish flag.
    // When true, publish the agent's calculated port to the host.
    // Fixed host-port overrides are a cast.json concern (config.port).
    if opts.publish {
        run_args.push("-p".to_string());
        run_args.push(format!("{}:80", opts.port));
    }

    // Env files.
    run_args.extend(build_env_file_args(
        &opts.workspace.root,
        opts.host_home_dir.as_deref(),
    ));

    // Environment: approval-gated host passthrough, as valueless `-e NAME`.
    // Docker applies --env after --env-file (last one wins), so these beat
    // cast.env wherever they sit; but against cast's own `-e USER=` below the
    // precedence is positional, so passthrough must come first to keep cast
    // authoritative for its own vars.
    run_args.extend(build_env_passthrough_args(
        &effective_env_passthrough(config),
        host_env_names,
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

    // Host identity: injected so diagnostics collected inside the container
    // can be grouped by the launching host. Always present (falls back to
    // "unknown" in resolve_run_opts when the hostname is unavailable).
    run_args.extend([
        "-e".to_string(),
        format!("CAST_HOST_NAME={}", opts.host_name),
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
            host_name: "test-host".to_string(),
            user_flake_present: false,
            project_flake_present: false,
            tty_mode: TtyMode::Interactive,
            publish: false,
        }
    }

    /// Host environment with no variables set, for the majority of flag tests
    /// that are not about passthrough.
    fn no_host_env() -> BTreeSet<String> {
        BTreeSet::new()
    }

    fn host_env(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    fn make_headless_opts(user: ResolvedUser, workspace: ResolvedWorkspace, port: u16) -> RunOpts {
        RunOpts {
            workspace,
            user,
            port,
            host_home_dir: Some(PathBuf::from("/home/alice")),
            host_name: "test-host".to_string(),
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

        let run_args = build_docker_run_flags(&config, &opts, &no_host_env());

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
        assert!(
            run_args.contains(&"CAST_MCP_URL=http://host.docker.internal:8080/mcp".to_string())
        );
    }

    #[test]
    fn test_build_docker_run_flags_env_passthrough_emitted_valueless() {
        let config = Config {
            env_passthrough: vec!["GH_TOKEN".to_string()],
            ..Config::default()
        };
        let opts = make_interactive_opts(alice_user(), alice_workspace(), 32768);

        let run_args = build_docker_run_flags(&config, &opts, &host_env(&["GH_TOKEN"]));

        assert!(run_args.contains(&"GH_TOKEN".to_string()));
        // No `NAME=value` form: the value must not reach cast's argv.
        assert!(
            !run_args.iter().any(|a| a.starts_with("GH_TOKEN=")),
            "passthrough emitted a valued arg: {run_args:?}"
        );
    }

    #[test]
    fn test_build_docker_run_flags_env_passthrough_omitted_when_unset_on_host() {
        let config = Config {
            env_passthrough: vec!["GH_TOKEN".to_string()],
            ..Config::default()
        };
        let opts = make_interactive_opts(alice_user(), alice_workspace(), 32768);

        let run_args = build_docker_run_flags(&config, &opts, &no_host_env());

        assert!(!run_args.contains(&"GH_TOKEN".to_string()));
    }

    #[test]
    fn test_build_docker_run_flags_env_passthrough_precedes_cast_infra_vars() {
        let config = Config {
            env_passthrough: vec!["GH_TOKEN".to_string()],
            ..Config::default()
        };
        let opts = make_interactive_opts(alice_user(), alice_workspace(), 32768);

        let run_args = build_docker_run_flags(&config, &opts, &host_env(&["GH_TOKEN"]));

        let passthrough = run_args.iter().position(|a| a == "GH_TOKEN").unwrap();
        let user = run_args.iter().position(|a| a == "USER=alice").unwrap();

        // Load-bearing: docker's last-occurrence-wins makes this ordering the
        // only thing keeping cast authoritative for the names it sets itself.
        // An allowlisted USER must not be able to override cast's own.
        assert!(
            passthrough < user,
            "passthrough must precede cast's infra vars: {run_args:?}"
        );
        // The `-e` belonging to the passthrough name sits immediately before it.
        assert_eq!(run_args[passthrough - 1], "-e");
    }

    #[test]
    fn test_effective_env_passthrough_concatenates_base_then_extra() {
        let config = Config {
            env_passthrough: vec!["BASE_ONE".to_string(), "BASE_TWO".to_string()],
            extra_env_passthrough: vec!["EXTRA_ONE".to_string()],
            ..Config::default()
        };

        assert_eq!(
            effective_env_passthrough(&config),
            vec![
                "BASE_ONE".to_string(),
                "BASE_TWO".to_string(),
                "EXTRA_ONE".to_string()
            ]
        );
    }

    #[test]
    fn test_effective_env_passthrough_empty_when_both_keys_empty() {
        assert!(effective_env_passthrough(&Config::default()).is_empty());
    }

    /// The reserved-name warning feeds from the effective list, so a
    /// reserved name smuggled into the extra key must be caught too.
    #[test]
    fn test_reserved_names_include_extra_list_entries() {
        let config = Config {
            extra_env_passthrough: vec!["GH_TOKEN".to_string(), "PATH".to_string()],
            ..Config::default()
        };

        let reserved = reserved_names_in(&effective_env_passthrough(&config));

        assert_eq!(reserved, BTreeSet::from(["PATH".to_string()]));
    }

    #[test]
    fn test_build_docker_run_flags_base_and_extra_both_emit_valueless() {
        let config = Config {
            env_passthrough: vec!["GH_TOKEN".to_string()],
            extra_env_passthrough: vec!["NPM_TOKEN".to_string()],
            ..Config::default()
        };
        let opts = make_interactive_opts(alice_user(), alice_workspace(), 32768);

        let run_args =
            build_docker_run_flags(&config, &opts, &host_env(&["GH_TOKEN", "NPM_TOKEN"]));

        assert!(run_args.contains(&"GH_TOKEN".to_string()));
        assert!(run_args.contains(&"NPM_TOKEN".to_string()));
        assert!(
            !run_args
                .iter()
                .any(|a| a.starts_with("GH_TOKEN=") || a.starts_with("NPM_TOKEN=")),
            "passthrough emitted a valued arg: {run_args:?}"
        );
    }

    /// A name allowlisted in both keys is one name: the dedupe pass runs
    /// over the concatenation, so it emits a single `-e` pair.
    #[test]
    fn test_build_docker_run_flags_cross_key_duplicate_emits_one_pair() {
        let config = Config {
            env_passthrough: vec!["GH_TOKEN".to_string()],
            extra_env_passthrough: vec!["GH_TOKEN".to_string()],
            ..Config::default()
        };
        let opts = make_interactive_opts(alice_user(), alice_workspace(), 32768);

        let run_args = build_docker_run_flags(&config, &opts, &host_env(&["GH_TOKEN"]));

        assert_eq!(
            run_args.iter().filter(|a| a.as_str() == "GH_TOKEN").count(),
            1,
            "duplicate across keys must emit one -e pair: {run_args:?}"
        );
    }

    #[test]
    fn test_build_docker_run_flags_mcp_custom_port() {
        let mut config = Config::default();
        config.mcp.port = 9000;
        let opts = make_interactive_opts(alice_user(), alice_workspace(), 32768);

        let run_args = build_docker_run_flags(&config, &opts, &no_host_env());
        assert!(
            run_args.contains(&"CAST_MCP_URL=http://host.docker.internal:9000/mcp".to_string())
        );
    }

    #[test]
    fn test_build_docker_run_flags_add_host_enabled() {
        let config = Config {
            add_host_docker_internal: true,
            ..Config::default()
        };
        let opts = make_interactive_opts(alice_user(), alice_workspace(), 32768);

        let run_args = build_docker_run_flags(&config, &opts, &no_host_env());

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

        let run_args = build_docker_run_flags(&config, &opts, &no_host_env());

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
        let run_args = build_docker_run_flags(&config, &opts, &no_host_env());

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
            publish: false,
        };
        let opts = resolve_run_opts(alice_user(), workspace, 8080, &flags);

        assert!(opts.project_flake_present);
        // user_flake_present depends on host home dir, which we can't easily mock here
        // but we verified the logic is correct.
    }

    #[test]
    fn test_resolve_run_opts_does_not_scaffold_global_flake() {
        // resolve_run_opts is pure detection: it must never write the global
        // flake to the host home. Point HOME at a fresh temp dir and assert
        // nothing is created there. Scaffolding is run_agent/exec's job.
        let home = tempfile::TempDir::new().unwrap();
        let workspace = ResolvedWorkspace {
            root: tempfile::TempDir::new().unwrap().path().to_path_buf(),
            container_path: PathBuf::from("/work"),
        };
        let flags = SessionFlags {
            mode: RunMode::Interactive,
            name: None,
            publish: false,
        };

        // Save/restore HOME around the call. Single-threaded assertion on our
        // own temp dir; we do not assert on any global state.
        let prev_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", home.path());
        }
        let _opts = resolve_run_opts(alice_user(), workspace, 8080, &flags);
        unsafe {
            match prev_home {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
        }

        assert!(
            !home.path().join(".config/cast/nix/flake.nix").exists(),
            "resolve_run_opts must not scaffold the global flake"
        );
        assert!(
            !home.path().join(".config/cast/cast.json").exists(),
            "resolve_run_opts must not scaffold the global cast.json"
        );
    }

    #[test]
    fn test_resolve_run_opts_populates_host_name() {
        // resolve_run_opts calls get_hostname() (real I/O). Assert only on
        // the invariant our sentinel guarantees: non-empty. Never assert a
        // specific hostname value.
        let temp = tempfile::TempDir::new().unwrap();
        let workspace = ResolvedWorkspace {
            root: temp.path().to_path_buf(),
            container_path: PathBuf::from("/work"),
        };
        let flags = SessionFlags {
            mode: RunMode::Interactive,
            name: None,
            publish: false,
        };
        let opts = resolve_run_opts(alice_user(), workspace, 8080, &flags);

        assert!(
            !opts.host_name.is_empty(),
            "host_name must be non-empty (real hostname or 'unknown' sentinel)"
        );
    }

    // ── Session run args: universal topology (run == exec) ──────────────────

    #[test]
    fn test_session_run_args_use_universal_topology() {
        let config = Config::default();
        let opts = make_interactive_opts(alice_user(), alice_workspace(), 32768);

        let args = build_session_run_args(&config, &opts).unwrap();

        // Shared cache/local volumes — NOT per-agent.
        assert!(
            args.contains(&"cast-cache:/home/alice/.cache:rw".to_string()),
            "expected shared cache volume, got: {args:?}"
        );
        assert!(
            args.contains(&"cast-local:/home/alice/.local:rw".to_string()),
            "expected shared local volume, got: {args:?}"
        );
        assert!(
            !args.iter().any(|a| a.contains("cast-opencode-cache")),
            "session must not use per-agent data volumes: {args:?}"
        );

        // Union of every agent's config mounts (opencode, claude, pi).
        assert!(
            args.iter().any(|a| a.contains("/.config/opencode:rw")),
            "opencode config mount missing: {args:?}"
        );
        assert!(
            args.iter().any(|a| a.contains("/.claude:rw")),
            "claude config mount missing: {args:?}"
        );
        assert!(
            args.iter().any(|a| a.contains("/.pi:rw")),
            "pi config mount missing: {args:?}"
        );
    }

    // ── CAST_HOST_NAME injection ──────────────────────────────────────────────

    #[test]
    fn test_build_docker_run_flags_injects_cast_host_name() {
        let config = Config::default();
        let opts = make_interactive_opts(alice_user(), alice_workspace(), 32768);
        let run_args = build_docker_run_flags(&config, &opts, &no_host_env());

        assert!(
            run_args.contains(&"CAST_HOST_NAME=test-host".to_string()),
            "Should inject CAST_HOST_NAME in interactive mode, got: {:?}",
            run_args
        );
    }

    #[test]
    fn test_build_docker_run_flags_headless_injects_cast_host_name() {
        let config = Config::default();
        let opts = make_headless_opts(alice_user(), alice_workspace(), 32768);
        let run_args = build_docker_run_flags(&config, &opts, &no_host_env());

        assert!(
            run_args.contains(&"CAST_HOST_NAME=test-host".to_string()),
            "Should inject CAST_HOST_NAME in headless mode, got: {:?}",
            run_args
        );
    }

    // ── Phase 2: build_docker_run_flags — headless mode ─────────────────────

    #[test]
    fn test_build_docker_run_flags_headless_no_tty_flags() {
        let config = Config::default();
        let opts = make_headless_opts(alice_user(), alice_workspace(), 32768);
        let run_args = build_docker_run_flags(&config, &opts, &no_host_env());

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
    fn test_build_docker_run_flags_no_publish_flag_no_port() {
        // publish: false (the default) → no -p flag regardless of tty mode.
        let config = Config::default();
        let opts = make_headless_opts(alice_user(), alice_workspace(), 32768);
        let run_args = build_docker_run_flags(&config, &opts, &no_host_env());

        assert!(
            !run_args.contains(&"-p".to_string()),
            "Should NOT publish port when publish is false"
        );
    }

    #[test]
    fn test_build_docker_run_flags_headless_no_color_vars() {
        let config = Config::default();
        let opts = make_headless_opts(alice_user(), alice_workspace(), 32768);
        let run_args = build_docker_run_flags(&config, &opts, &no_host_env());

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
        let run_args = build_docker_run_flags(&config, &opts, &no_host_env());

        assert!(
            run_args.contains(&"NO_COLOR=1".to_string()),
            "Should inject NO_COLOR=1 in headless mode"
        );
    }

    #[test]
    fn test_build_docker_run_flags_headless_user_present() {
        let config = Config::default();
        let opts = make_headless_opts(alice_user(), alice_workspace(), 32768);
        let run_args = build_docker_run_flags(&config, &opts, &no_host_env());

        assert!(
            run_args.contains(&"USER=alice".to_string()),
            "USER= must be present in headless mode"
        );
    }

    #[test]
    fn test_build_docker_run_flags_publish_true_emits_port() {
        // publish: true → -p <port>:80 with the calculated port.
        let config = Config::default();
        let opts = RunOpts {
            publish: true,
            host_name: "test-host".to_string(),
            ..make_interactive_opts(alice_user(), alice_workspace(), 32768)
        };
        let run_args = build_docker_run_flags(&config, &opts, &no_host_env());

        let p_pos = run_args.iter().position(|a| a == "-p");
        assert!(p_pos.is_some(), "Should contain -p when publish is true");
        assert_eq!(
            run_args[p_pos.unwrap() + 1],
            "32768:80",
            "publish: true should emit the calculated port"
        );
    }

    #[test]
    fn test_build_docker_run_flags_no_publish_by_default() {
        // publish: false → no -p regardless of tty mode.
        let config = Config::default();
        let opts = make_interactive_opts(alice_user(), alice_workspace(), 32768);
        let run_args = build_docker_run_flags(&config, &opts, &no_host_env());

        assert!(
            !run_args.contains(&"-p".to_string()),
            "Should NOT publish port when publish is false (default)"
        );
    }
}
