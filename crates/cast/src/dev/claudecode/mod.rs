pub mod config_dir;

use anyhow::{Context, Result};

use crate::config::Config;
use crate::dev::agent::Agent;
use crate::dev::run::RunOpts;

/// The ClaudeCode agent — runs the `claude` program inside the dev container.
pub struct ClaudeCode;

impl Agent for ClaudeCode {
    fn name(&self) -> &'static str {
        "claudecode"
    }

    fn prepare_host(&self, _config: &Config, opts: &RunOpts) -> Result<()> {
        let home = opts
            .host_home_dir
            .as_deref()
            .context("Failed to resolve user home directory")?;
        config_dir::ensure_config_dir(home)?;
        config_dir::ensure_config_file(home)?;
        Ok(())
    }

    fn base_command(&self) -> &'static str {
        "claude"
    }

    fn config_mount_args(&self, _config: &Config, opts: &RunOpts) -> Result<Vec<String>> {
        let home = opts
            .host_home_dir
            .as_deref()
            .context("Failed to resolve user home directory")?;

        let claude_config_dir = config_dir::get_config_dir(home);
        let claude_config_file = config_dir::get_config_file(home);

        Ok(vec![
            // ~/.claude directory
            "-v".to_string(),
            format!(
                "{}:/home/{}/.claude:rw",
                claude_config_dir.display(),
                opts.user.username
            ),
            // ~/.claude.json global config file
            "-v".to_string(),
            format!(
                "{}:/home/{}/.claude.json:rw",
                claude_config_file.display(),
                opts.user.username
            ),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::dev::run::RunOpts;
    use crate::dev::workspace::ResolvedWorkspace;
    use crate::user::ResolvedUser;
    use std::path::PathBuf;

    fn testuser() -> ResolvedUser {
        ResolvedUser {
            username: "testuser".to_string(),
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
            user: testuser(),
            port: 32768,
            host_home_dir: Some(PathBuf::from("/home/testuser")),
            host_name: "test-host".to_string(),
            tty_mode: crate::dev::run::TtyMode::Interactive,
            publish: false,
        }
    }

    #[test]
    fn test_config_mount_args_includes_claude_config_mount() {
        let config = Config::default();
        let opts = basic_opts(PathBuf::from("/tmp/workspace"));

        let args = ClaudeCode.config_mount_args(&config, &opts).unwrap();

        assert!(
            args.contains(&"/home/testuser/.claude:/home/testuser/.claude:rw".to_string()),
            "expected claude config bind mount in args: {:?}",
            args
        );
    }

    #[test]
    fn test_config_mount_args_includes_claude_json_mount() {
        let config = Config::default();
        let opts = basic_opts(PathBuf::from("/tmp/workspace"));

        let args = ClaudeCode.config_mount_args(&config, &opts).unwrap();

        assert!(
            args.contains(
                &"/home/testuser/.claude.json:/home/testuser/.claude.json:rw".to_string()
            ),
            "expected claude.json bind mount in args: {:?}",
            args
        );
    }
}
