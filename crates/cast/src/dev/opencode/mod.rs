pub mod config_dir;
pub mod env;

use std::collections::HashMap;

use anyhow::{Context, Result};

use crate::config::Config;
use crate::dev::agent::Agent;
use crate::dev::run::RunOpts;

/// The OpenCode agent — runs the `opencode` program inside the dev container.
pub struct OpenCode;

impl Agent for OpenCode {
    fn name(&self) -> &'static str {
        "opencode"
    }

    fn prepare_host(&self, _config: &Config, opts: &RunOpts) -> Result<()> {
        let home = opts
            .host_home_dir
            .as_deref()
            .context("Failed to resolve user home directory")?;
        config_dir::ensure_config_dir(&home.join(".config"))?;
        Ok(())
    }

    fn base_command(&self) -> &'static str {
        "opencode"
    }

    fn config_mount_args(&self, _config: &Config, opts: &RunOpts) -> Result<Vec<String>> {
        let home = opts
            .host_home_dir
            .as_deref()
            .context("Failed to resolve user home directory")?;
        let opencode_config_dir = config_dir::get_config_dir(&home.join(".config"));

        // Skip if the workspace root is the same as the config dir (workspace
        // mount covers it).
        if opencode_config_dir == opts.workspace.root {
            return Ok(vec![]);
        }

        Ok(vec![
            "-v".to_string(),
            format!(
                "{}:/home/{}/.config/opencode:rw",
                opencode_config_dir.display(),
                opts.user.username
            ),
        ])
    }

    fn env_passthrough_args(&self, env: &HashMap<String, String>) -> Vec<String> {
        env::build_passthrough_env_args(env)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dev::run::RunOpts;
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

    fn basic_opts(workspace_root: PathBuf) -> RunOpts {
        RunOpts {
            workspace: ResolvedWorkspace {
                container_path: workspace_root.clone(),
                root: workspace_root,
            },
            user: alice(),
            port: 32768,
            host_home_dir: Some(PathBuf::from("/home/alice")),
            host_name: "test-host".to_string(),
            user_flake_present: false,
            project_flake_present: false,
            tty_mode: crate::dev::run::TtyMode::Interactive,
            publish: false,
        }
    }

    // --- extra_run_args ---

    #[test]
    fn test_prepare_host_creates_config_dir() {
        let temp = tempfile::TempDir::new().unwrap();
        let config_home = temp.path().to_path_buf();

        // We test ensure_config_dir directly with a temp path to avoid unsafe env mutation.
        let result = config_dir::ensure_config_dir(&config_home).unwrap();

        assert!(result.exists());
        assert_eq!(result, config_home.join("opencode"));
    }

    #[test]
    fn test_config_mount_args_includes_opencode_config_dir() {
        let config = Config::default();
        let opts = basic_opts(PathBuf::from("/home/alice/project"));

        let args = OpenCode.config_mount_args(&config, &opts).unwrap();

        assert!(
            args.contains(
                &"/home/alice/.config/opencode:/home/alice/.config/opencode:rw".to_string()
            ),
            "expected opencode config bind mount: {args:?}"
        );
    }

    #[test]
    fn test_config_mount_args_workspace_conflict_no_double_mount() {
        let config = Config::default();
        // workspace root == opencode config dir → no duplicate mount
        let workspace_root = PathBuf::from("/home/alice/.config/opencode");

        let opts = RunOpts {
            workspace: ResolvedWorkspace {
                container_path: workspace_root.clone(),
                root: workspace_root,
            },
            user: alice(),
            port: 32768,
            host_home_dir: Some(PathBuf::from("/home/alice")),
            host_name: "test-host".to_string(),
            user_flake_present: false,
            project_flake_present: false,
            tty_mode: crate::dev::run::TtyMode::Interactive,
            publish: false,
        };

        let args = OpenCode.config_mount_args(&config, &opts).unwrap();

        // The opencode config dir mount must not appear (workspace covers it).
        let mount_count = args
            .iter()
            .filter(|a| a.contains("/.config/opencode:rw"))
            .count();
        assert_eq!(mount_count, 0);
    }
}
