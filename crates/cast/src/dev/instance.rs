use std::collections::BTreeSet;
use std::time::Duration;

use anyhow::Result;

use crate::config::{ApprovedConfig, Config};
use crate::dev::instance_context::InstanceContext;
use crate::dev::run::{RunMode, RunOpts, SessionFlags, build_instance_run_flags, resolve_run_opts};
use crate::docker::BuildOptions;
use crate::docker::args::{build_remove_args, build_run_args, build_stop_args};
use crate::docker::client::DockerClient;
use crate::user::get_user;
use crate::{dev, nix_daemon};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceStatus {
    Absent,
    Stopped,
    Running,
}

fn classify_instance_status(exists: bool, running: bool) -> InstanceStatus {
    match (exists, running) {
        (false, _) => InstanceStatus::Absent,
        (true, false) => InstanceStatus::Stopped,
        (true, true) => InstanceStatus::Running,
    }
}

impl std::fmt::Display for InstanceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            InstanceStatus::Absent => "absent",
            InstanceStatus::Stopped => "stopped",
            InstanceStatus::Running => "running",
        })
    }
}

pub fn build_instance_command() -> Vec<String> {
    vec!["sleep".to_string(), "infinity".to_string()]
}

/// Prefix of every Docker label cast sets; reverse DNS of palekiwi.com.
pub const LABEL_PREFIX: &str = "com.palekiwi.cast";
/// Version of the label set; bump when keys change meaning.
pub const LABEL_SCHEMA: &str = "1";

/// Labels that identify an instance container to `cast ps` and other host
/// tools. The name label is omitted for the default instance.
fn build_instance_label_args(
    context: &InstanceContext,
    instance_name: Option<&str>,
) -> Vec<String> {
    let mut labels = vec![
        ("schema", LABEL_SCHEMA.to_string()),
        ("workspace", context.worktree_root.display().to_string()),
        ("workspace_id", context.workspace_id.clone()),
        ("repo", context.git_common_dir.display().to_string()),
        ("version", env!("CARGO_PKG_VERSION").to_string()),
    ];
    if let Some(name) = instance_name {
        labels.push(("name", name.to_string()));
    }
    labels
        .into_iter()
        .flat_map(|(key, value)| {
            [
                "--label".to_string(),
                format!("{LABEL_PREFIX}.{key}={value}"),
            ]
        })
        .collect()
}

pub fn build_instance_docker_args(
    config: &Config,
    opts: &RunOpts,
    context: &InstanceContext,
    instance_name: Option<&str>,
    image_tag: &str,
    host_env_names: &BTreeSet<String>,
) -> Vec<String> {
    let container_name = context.container_name(instance_name);
    let container_workdir = context.container_workdir(&opts.workspace);
    let mut flags = build_instance_run_flags(
        config,
        opts,
        host_env_names,
        &context.git_common_dir,
        &container_workdir,
    );
    flags.extend(build_instance_label_args(context, instance_name));
    let command = build_instance_command();

    build_run_args(&container_name, image_tag, flags, Some(command))
}

fn build_instance_up_cleanup_command(
    context: &InstanceContext,
    instance_name: Option<&str>,
    running: bool,
) -> Option<Vec<String>> {
    (!running).then(|| build_remove_args(&context.container_name(instance_name)))
}

pub fn up(
    config: &ApprovedConfig,
    context: &InstanceContext,
    instance_name: Option<&str>,
) -> Result<()> {
    let docker = DockerClient;
    let container_name = context.container_name(instance_name);
    if let Some(inspection) = docker.inspect_container(&container_name)? {
        if inspection.running {
            eprintln!("instance is already running: {container_name}");
            return Ok(());
        }
        if let Some(command) = build_instance_up_cleanup_command(context, instance_name, false) {
            docker.run_command(command)?;
        }
    }

    let user = get_user()?;
    let workspace = context.workspace(dirs::home_dir().as_deref(), &user.username);
    let run_opts = resolve_run_opts(
        user,
        workspace,
        0,
        &SessionFlags {
            mode: RunMode::Headless {
                token: "instance".to_string(),
            },
            name: instance_name.map(str::to_string),
            publish: false,
        },
    );

    dev::volumes::validate_bind_sources(
        config,
        &run_opts.user,
        &run_opts.workspace,
        run_opts.host_home_dir.as_deref(),
    )?;

    nix_daemon::ensure_running(&docker, config)?;
    dev::image::ensure_dev_image(&docker, config, &run_opts.user, BuildOptions::default())?;

    let host_env_names = std::env::vars()
        .filter(|(_, value)| !value.is_empty())
        .map(|(name, _)| name)
        .collect::<BTreeSet<_>>();
    let image_tag = dev::image::image_tag();
    let args = build_instance_docker_args(
        config,
        &run_opts,
        context,
        instance_name,
        &image_tag,
        &host_env_names,
    );
    docker.run_command(args)?;
    eprintln!("instance running: {container_name}");

    Ok(())
}

pub fn down(context: &InstanceContext, instance_name: Option<&str>) -> Result<()> {
    let docker = DockerClient;
    let container_name = context.container_name(instance_name);
    if !docker.container_exists(&container_name)? {
        eprintln!("instance is already absent: {container_name}");
        return Ok(());
    }

    if docker.is_container_running(&container_name)? {
        docker.run_command(build_stop_args(&container_name))?;
    }
    // Stopping auto-removes the container (`--rm`); the explicit removal
    // covers containers created without it, and the wait frees the name
    // for an immediate `cast up`.
    docker.remove_container(&container_name)?;
    docker.wait_until_absent(&container_name, Duration::from_secs(10))?;
    eprintln!("instance removed: {container_name}");

    Ok(())
}

pub fn status(context: &InstanceContext, instance_name: Option<&str>) -> Result<InstanceStatus> {
    let docker = DockerClient;
    let container_name = context.container_name(instance_name);
    let inspection = docker.inspect_container(&container_name)?;
    Ok(classify_instance_status(
        inspection.is_some(),
        inspection.is_some_and(|state| state.running),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dev::instance_context::InstanceContext;
    use crate::dev::run::{RunOpts, TtyMode};
    use crate::dev::workspace::ResolvedWorkspace;
    use crate::user::ResolvedUser;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    #[test]
    fn instance_command_is_a_vendor_neutral_keepalive() {
        let command = build_instance_command();

        assert_eq!(command, vec!["sleep", "infinity"]);
    }

    #[test]
    fn instance_docker_args_target_the_named_worktree_instance() {
        let config = Config::default();
        let context = InstanceContext {
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

        let args = build_instance_docker_args(
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
        assert!(args.ends_with(&[
            "localhost/cast:spike".to_string(),
            "sleep".to_string(),
            "infinity".to_string()
        ]));
    }

    #[test]
    fn instance_containers_carry_identifying_labels() {
        let context = InstanceContext {
            worktree_root: PathBuf::from("/home/alice/projects/my-app"),
            git_common_dir: PathBuf::from("/home/alice/main/.git"),
            relative_cwd: PathBuf::new(),
            workspace_id: "a1b2c3d4e5f6".to_string(),
        };
        let version = env!("CARGO_PKG_VERSION");

        assert_eq!(
            build_instance_label_args(&context, Some("qa")),
            vec![
                "--label".to_string(),
                "com.palekiwi.cast.schema=1".to_string(),
                "--label".to_string(),
                "com.palekiwi.cast.workspace=/home/alice/projects/my-app".to_string(),
                "--label".to_string(),
                "com.palekiwi.cast.workspace_id=a1b2c3d4e5f6".to_string(),
                "--label".to_string(),
                "com.palekiwi.cast.repo=/home/alice/main/.git".to_string(),
                "--label".to_string(),
                format!("com.palekiwi.cast.version={version}"),
                "--label".to_string(),
                "com.palekiwi.cast.name=qa".to_string(),
            ]
        );
        assert!(
            !build_instance_label_args(&context, None)
                .iter()
                .any(|arg| arg.starts_with("com.palekiwi.cast.name=")),
            "the default instance carries no name label"
        );
    }

    #[test]
    fn instance_status_reports_container_lifecycle() {
        assert_eq!(
            classify_instance_status(false, false),
            InstanceStatus::Absent
        );
        assert_eq!(
            classify_instance_status(true, false),
            InstanceStatus::Stopped
        );
        assert_eq!(
            classify_instance_status(true, true),
            InstanceStatus::Running
        );
    }

    #[test]
    fn instance_up_removes_a_stopped_container_before_recreating_it() {
        let context = InstanceContext {
            worktree_root: PathBuf::from("/home/alice/projects/my-app"),
            git_common_dir: PathBuf::from("/home/alice/projects/my-app/.git"),
            relative_cwd: PathBuf::new(),
            workspace_id: "a1b2c3d4e5f6".to_string(),
        };

        assert_eq!(
            build_instance_up_cleanup_command(&context, Some("isolated"), false),
            Some(vec![
                "rm".to_string(),
                "cast-my-app-a1b2c3d4e5f6-isolated".to_string()
            ])
        );
        assert_eq!(
            build_instance_up_cleanup_command(&context, Some("isolated"), true),
            None
        );
    }
}
