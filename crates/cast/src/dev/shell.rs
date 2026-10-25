use std::path::Path;
use std::process::ExitStatus;

use anyhow::Result;

use crate::config::{ApprovedConfig, Config};
use crate::dev::exec::{InstanceExecOptions, build_instance_exec_args};
use crate::dev::instance_context::InstanceContext;

/// The shell an instance session opens inside the container.
const INSTANCE_SHELL: &str = "/bin/bash";

/// Build `docker exec` arguments for an interactive shell in an instance.
pub fn build_instance_shell_args(
    config: &Config,
    context: &InstanceContext,
    instance_name: Option<&str>,
    container_username: &str,
    container_workdir: &Path,
    raw: bool,
    host_term: Option<&str>,
) -> Vec<String> {
    build_instance_exec_args(
        config,
        context,
        &InstanceExecOptions {
            instance_name,
            container_username,
            container_workdir,
            headless: false,
            raw,
            host_term,
        },
        &[INSTANCE_SHELL.to_string()],
    )
}

/// Drop into an interactive shell in the selected worktree instance.
pub fn shell(
    config: &ApprovedConfig,
    context: &InstanceContext,
    instance_name: Option<&str>,
    raw: bool,
) -> Result<ExitStatus> {
    crate::dev::exec(
        config,
        context,
        instance_name,
        false,
        raw,
        vec![INSTANCE_SHELL.to_string()],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::dev::instance_context::InstanceContext;
    use std::path::{Path, PathBuf};

    #[test]
    fn instance_shell_opens_an_interactive_wrapped_shell() {
        let config = Config {
            sandbox_shell: Some("~/.config/cast/nix#default".to_string()),
            project_shell: Some(".#ai".to_string()),
            ..Config::default()
        };
        let context = InstanceContext {
            worktree_root: PathBuf::from("/home/alice/projects/my-app"),
            git_common_dir: PathBuf::from("/home/alice/projects/my-app/.git"),
            relative_cwd: PathBuf::new(),
            workspace_id: "a1b2c3d4e5f6".to_string(),
        };

        let args = build_instance_shell_args(
            &config,
            &context,
            Some("review"),
            "alice",
            Path::new("/home/alice/projects/my-app"),
            false,
            Some("tmux"),
        );

        assert_eq!(
            args,
            vec![
                "exec",
                "-it",
                "-e",
                "TERM=tmux",
                "-e",
                "COLORTERM=truecolor",
                "-e",
                "FORCE_COLOR=1",
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
