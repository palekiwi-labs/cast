use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use anyhow::{bail, Result};

use crate::config::{ApprovedConfig, Config};
use crate::dev::build_command::build_sandbox_command;
use crate::dev::run::{build_service_run_flags, resolve_run_opts, RunMode, RunOpts, SessionFlags};
use crate::dev::service_context::ServiceContext;
use crate::docker::args::{build_remove_args, build_run_args, build_stop_args};
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::user::get_user;
use crate::{dev, nix_daemon};

const PROVISIONING_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const READINESS_TIMEOUT: Duration = Duration::from_secs(30);
const STARTUP_POLL_INTERVAL: Duration = Duration::from_millis(250);
const STARTUP_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const READINESS_PROBE_TIMEOUT: &str = "10s";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    Absent,
    Stopped,
    Running,
}

fn classify_service_status(exists: bool, running: bool) -> ServiceStatus {
    match (exists, running) {
        (false, _) => ServiceStatus::Absent,
        (true, false) => ServiceStatus::Stopped,
        (true, true) => ServiceStatus::Running,
    }
}

fn service_process_started(processes: &str) -> bool {
    processes.lines().any(|line| {
        let mut words = line.split_whitespace();
        if !words.next().is_some_and(|pid| pid.parse::<u32>().is_ok()) {
            return false;
        }
        let executable = words.next().and_then(|word| word.rsplit('/').next());
        executable == Some("herdr") && words.next() == Some("server")
    })
}

fn process_snapshot_changed(previous: &mut Option<String>, current: &str) -> bool {
    if previous.as_deref() == Some(current) {
        return false;
    }
    *previous = Some(current.to_string());
    true
}

fn next_heartbeat_after(elapsed: Duration) -> Duration {
    elapsed + STARTUP_HEARTBEAT_INTERVAL
}

impl std::fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ServiceStatus::Absent => "absent",
            ServiceStatus::Stopped => "stopped",
            ServiceStatus::Running => "running",
        })
    }
}

pub fn build_service_command(config: &Config, username: &str) -> Vec<String> {
    build_sandbox_command(config, username, "herdr", vec!["server".to_string()])
}

pub fn build_service_readiness_probe_args(
    config: &Config,
    context: &ServiceContext,
    service_name: Option<&str>,
    username: &str,
) -> Vec<String> {
    let mut args = vec![
        "exec".to_string(),
        "-e".to_string(),
        format!(
            "HERDR_SESSION={}",
            context.multiplexer_session_name(service_name)
        ),
        context.container_name(service_name),
        "timeout".to_string(),
        "--signal=TERM".to_string(),
        "--kill-after=1s".to_string(),
        READINESS_PROBE_TIMEOUT.to_string(),
    ];
    args.extend(build_sandbox_command(
        config,
        username,
        "herdr",
        vec!["api".to_string(), "snapshot".to_string()],
    ));
    args
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

fn exited_during_startup(
    docker: &DockerClient,
    container_name: &str,
    stage: &str,
) -> anyhow::Error {
    let logs = docker
        .container_logs(container_name, 100)
        .unwrap_or_else(|error| format!("failed to read container logs: {error}"));
    anyhow::anyhow!(
        "service container exited during {stage}: {container_name}\n\nrecent logs:\n{}",
        logs.trim()
    )
}

fn wait_for_service_process(docker: &DockerClient, container_name: &str) -> Result<()> {
    let started = Instant::now();
    let mut previous_processes = None;
    let mut next_heartbeat = STARTUP_HEARTBEAT_INTERVAL;
    loop {
        if !docker.is_container_running(container_name)? {
            return Err(exited_during_startup(
                docker,
                container_name,
                "sandbox provisioning",
            ));
        }
        let processes = docker.container_processes(container_name)?;
        if process_snapshot_changed(&mut previous_processes, &processes) {
            eprintln!(
                "service process snapshot after {}s:\n{}",
                started.elapsed().as_secs(),
                processes.trim()
            );
        }
        if service_process_started(&processes) {
            return Ok(());
        }
        if started.elapsed() >= PROVISIONING_TIMEOUT {
            let logs = docker
                .container_logs(container_name, 100)
                .unwrap_or_else(|error| format!("failed to read container logs: {error}"));
            bail!(
                "timed out after {}s waiting for sandbox provisioning: {container_name}\n\n\
                 final process snapshot:\n{}\n\nrecent logs:\n{}",
                PROVISIONING_TIMEOUT.as_secs(),
                processes.trim(),
                logs.trim()
            );
        }
        if started.elapsed() >= next_heartbeat {
            let elapsed = started.elapsed();
            eprintln!(
                "still waiting for sandbox provisioning after {}s: {container_name}",
                elapsed.as_secs()
            );
            next_heartbeat = next_heartbeat_after(elapsed);
        }
        std::thread::sleep(STARTUP_POLL_INTERVAL);
    }
}

fn wait_for_service_api(
    docker: &DockerClient,
    config: &Config,
    context: &ServiceContext,
    service_name: Option<&str>,
    username: &str,
) -> Result<()> {
    let container_name = context.container_name(service_name);
    let probe_args = build_service_readiness_probe_args(config, context, service_name, username);
    let started = Instant::now();
    loop {
        if !docker.is_container_running(&container_name)? {
            return Err(exited_during_startup(
                docker,
                &container_name,
                "multiplexer readiness",
            ));
        }
        if docker.command_succeeds(probe_args.clone())? {
            return Ok(());
        }
        if started.elapsed() >= READINESS_TIMEOUT {
            bail!(
                "timed out after {}s waiting for multiplexer readiness: {container_name}",
                READINESS_TIMEOUT.as_secs()
            );
        }
        std::thread::sleep(STARTUP_POLL_INTERVAL);
    }
}

fn wait_for_service_ready(
    docker: &DockerClient,
    config: &Config,
    context: &ServiceContext,
    service_name: Option<&str>,
    username: &str,
) -> Result<()> {
    let container_name = context.container_name(service_name);
    eprintln!("waiting for sandbox provisioning: {container_name}");
    let _log_follower = docker.follow_container_logs(&container_name)?;
    wait_for_service_process(docker, &container_name)?;
    eprintln!("waiting for multiplexer readiness: {container_name}");
    wait_for_service_api(docker, config, context, service_name, username)
}

fn build_service_down_commands(
    context: &ServiceContext,
    service_name: Option<&str>,
    running: bool,
) -> Vec<Vec<String>> {
    let container_name = context.container_name(service_name);
    let mut commands = Vec::with_capacity(if running { 2 } else { 1 });
    if running {
        commands.push(build_stop_args(&container_name));
    }
    commands.push(build_remove_args(&container_name));
    commands
}

pub fn up(
    config: &ApprovedConfig,
    context: &ServiceContext,
    service_name: Option<&str>,
) -> Result<()> {
    let docker = DockerClient;
    let container_name = context.container_name(service_name);
    let user = get_user()?;
    if docker.is_container_running(&container_name)? {
        eprintln!("service is already running: {container_name}");
        wait_for_service_ready(&docker, config, context, service_name, &user.username)?;
        eprintln!("service ready: {container_name}");
        return Ok(());
    }

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
    wait_for_service_ready(
        &docker,
        config,
        context,
        service_name,
        &run_opts.user.username,
    )?;
    eprintln!("service ready: {container_name}");

    Ok(())
}

pub fn down(context: &ServiceContext, service_name: Option<&str>) -> Result<()> {
    let docker = DockerClient;
    let container_name = context.container_name(service_name);
    if !docker.container_exists(&container_name)? {
        eprintln!("service is already absent: {container_name}");
        return Ok(());
    }

    let running = docker.is_container_running(&container_name)?;
    for command in build_service_down_commands(context, service_name, running) {
        docker.run_command(command)?;
    }
    eprintln!("service removed: {container_name}");

    Ok(())
}

pub fn status(context: &ServiceContext, service_name: Option<&str>) -> Result<ServiceStatus> {
    let docker = DockerClient;
    let container_name = context.container_name(service_name);
    let exists = docker.container_exists(&container_name)?;
    if !exists {
        return Ok(classify_service_status(false, false));
    }

    let running = docker.is_container_running(&container_name)?;
    Ok(classify_service_status(true, running))
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
    fn readiness_probe_uses_the_service_session_in_the_sandbox_shell() {
        let config = Config {
            sandbox_shell: Some("~/.config/cast/nix#default".to_string()),
            project_shell: Some(".#project".to_string()),
            ..Default::default()
        };
        let context = ServiceContext {
            worktree_root: PathBuf::from("/home/alice/projects/my-app"),
            git_common_dir: PathBuf::from("/home/alice/projects/my-app/.git"),
            relative_cwd: PathBuf::new(),
            workspace_id: "a1b2c3d4e5f6".to_string(),
        };

        assert_eq!(
            build_service_readiness_probe_args(&config, &context, Some("isolated"), "alice"),
            vec![
                "exec",
                "-e",
                "HERDR_SESSION=cast-a1b2c3d4e5f6-isolated",
                "cast-my-app-a1b2c3d4e5f6-isolated",
                "timeout",
                "--signal=TERM",
                "--kill-after=1s",
                "10s",
                "nix",
                "develop",
                "/home/alice/.config/cast/nix#default",
                "-c",
                "herdr",
                "api",
                "snapshot",
            ]
        );
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

    #[test]
    fn service_status_reports_container_lifecycle() {
        assert_eq!(classify_service_status(false, false), ServiceStatus::Absent);
        assert_eq!(classify_service_status(true, false), ServiceStatus::Stopped);
        assert_eq!(classify_service_status(true, true), ServiceStatus::Running);
    }

    #[test]
    fn service_process_appears_only_after_nix_hands_off() {
        let provisioning = "PID COMMAND\n1 /sbin/docker-init -- nix develop shell -c herdr server\n7 nix develop shell -c herdr server\n";
        let started = "PID COMMAND\n1 /sbin/docker-init -- nix develop shell -c herdr server\n42 /nix/store/hash-herdr/bin/herdr server\n";

        assert!(!service_process_started(provisioning));
        assert!(service_process_started(started));
    }

    #[test]
    fn provisioning_diagnostics_report_only_process_changes() {
        let mut previous = None;

        assert!(process_snapshot_changed(
            &mut previous,
            "PID COMMAND\n1 nix develop shell -c herdr server\n"
        ));
        assert!(!process_snapshot_changed(
            &mut previous,
            "PID COMMAND\n1 nix develop shell -c herdr server\n"
        ));
        assert!(process_snapshot_changed(
            &mut previous,
            "PID COMMAND\n1 herdr server\n"
        ));
    }

    #[test]
    fn provisioning_heartbeat_reschedules_from_delayed_poll() {
        assert_eq!(
            next_heartbeat_after(Duration::from_secs(10 * 60)),
            Duration::from_secs(10 * 60 + 30)
        );
    }

    #[test]
    fn service_down_stops_then_removes_the_named_running_container() {
        let context = ServiceContext {
            worktree_root: PathBuf::from("/home/alice/projects/my-app"),
            git_common_dir: PathBuf::from("/home/alice/projects/my-app/.git"),
            relative_cwd: PathBuf::new(),
            workspace_id: "a1b2c3d4e5f6".to_string(),
        };

        assert_eq!(
            build_service_down_commands(&context, Some("isolated"), true),
            vec![
                vec!["stop", "cast-my-app-a1b2c3d4e5f6-isolated"],
                vec!["rm", "cast-my-app-a1b2c3d4e5f6-isolated"],
            ]
        );
    }

    #[test]
    fn service_down_only_removes_an_already_stopped_container() {
        let context = ServiceContext {
            worktree_root: PathBuf::from("/home/alice/projects/my-app"),
            git_common_dir: PathBuf::from("/home/alice/projects/my-app/.git"),
            relative_cwd: PathBuf::new(),
            workspace_id: "a1b2c3d4e5f6".to_string(),
        };

        assert_eq!(
            build_service_down_commands(&context, None, false),
            vec![vec!["rm", "cast-my-app-a1b2c3d4e5f6"]]
        );
    }
}
