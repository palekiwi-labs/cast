use std::path::Path;
use std::process::ExitStatus;

use anyhow::Result;

use crate::config::{ApprovedConfig, Config};
use crate::dev::exec::{ServiceExecOptions, build_service_exec_args};
use crate::dev::service_context::ServiceContext;

/// The shell a service session opens inside the container.
const SERVICE_SHELL: &str = "/bin/bash";

/// Build `docker exec` arguments for an interactive shell in a service.
pub fn build_service_shell_args(
    config: &Config,
    context: &ServiceContext,
    service_name: Option<&str>,
    container_username: &str,
    container_workdir: &Path,
    raw: bool,
) -> Vec<String> {
    build_service_exec_args(
        config,
        context,
        &ServiceExecOptions {
            service_name,
            container_username,
            container_workdir,
            headless: false,
            raw,
        },
        &[SERVICE_SHELL.to_string()],
    )
}

/// Drop into an interactive shell in the selected worktree service.
pub fn shell(
    config: &ApprovedConfig,
    context: &ServiceContext,
    service_name: Option<&str>,
    raw: bool,
) -> Result<ExitStatus> {
    crate::dev::exec(
        config,
        context,
        service_name,
        false,
        raw,
        vec![SERVICE_SHELL.to_string()],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::dev::service_context::ServiceContext;
    use std::path::{Path, PathBuf};

    #[test]
    fn service_shell_opens_an_interactive_wrapped_shell() {
        let config = Config {
            sandbox_shell: Some("~/.config/cast/nix#default".to_string()),
            project_shell: Some(".#ai".to_string()),
            ..Config::default()
        };
        let context = ServiceContext {
            worktree_root: PathBuf::from("/home/alice/projects/my-app"),
            git_common_dir: PathBuf::from("/home/alice/projects/my-app/.git"),
            relative_cwd: PathBuf::new(),
            workspace_id: "a1b2c3d4e5f6".to_string(),
        };

        let args = build_service_shell_args(
            &config,
            &context,
            Some("review"),
            "alice",
            Path::new("/home/alice/projects/my-app"),
            false,
        );

        assert_eq!(
            args,
            vec![
                "exec",
                "-it",
                "--workdir",
                "/home/alice/projects/my-app",
                "cast-my-app-a1b2c3d4e5f6-review",
                "nix",
                "develop",
                "/home/alice/.config/cast/nix#default",
                "-c",
                "nix",
                "develop",
                ".#ai",
                "-c",
                "/bin/bash",
            ]
        );
    }
}
