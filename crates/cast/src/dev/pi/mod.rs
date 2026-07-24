use anyhow::{Context, Result};

use crate::config::Config;
use crate::dev::agent::Agent;
use crate::dev::run::RunOpts;
use std::collections::HashMap;

pub mod config_dir;
pub mod env;

pub struct Pi;

impl Agent for Pi {
    fn name(&self) -> &'static str {
        "pi"
    }

    fn prepare_host(&self, _config: &Config, opts: &RunOpts) -> Result<()> {
        let home = opts
            .host_home_dir
            .as_deref()
            .context("Failed to resolve user home directory")?;
        config_dir::ensure_config_dir(home)?;
        Ok(())
    }

    fn base_command(&self) -> &'static str {
        "pi"
    }

    fn config_mount_args(&self, _config: &Config, opts: &RunOpts) -> Result<Vec<String>> {
        let home = opts
            .host_home_dir
            .as_deref()
            .context("Failed to resolve user home directory")?;
        let pi_config_host_dir = config_dir::get_config_dir(home);

        Ok(vec![
            "-v".to_string(),
            format!(
                "{}:/home/{}/.pi:rw",
                pi_config_host_dir.display(),
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
    use crate::config::Config;
    use crate::user::ResolvedUser;

    #[test]
    fn test_config_mount_args_includes_pi_config_dir() {
        let config = Config::default();
        let user = ResolvedUser {
            uid: 1000,
            gid: 1000,
            username: "testuser".to_string(),
        };
        let run_opts = RunOpts {
            user,
            workspace: crate::dev::workspace::ResolvedWorkspace {
                root: std::path::PathBuf::from("/tmp/workspace"),
                container_path: std::path::PathBuf::from("/workspace/tmp/workspace"),
            },
            port: 8080,
            host_home_dir: Some(std::path::PathBuf::from("/home/testuser")),
            host_name: "test-host".to_string(),
            user_flake_present: false,
            project_flake_present: false,
            tty_mode: crate::dev::run::TtyMode::Interactive,
            publish: false,
        };

        let pi = Pi;
        let args = pi.config_mount_args(&config, &run_opts).unwrap();

        assert!(args.contains(&"-v".to_string()));
        assert!(args.contains(&"/home/testuser/.pi:/home/testuser/.pi:rw".to_string()));
    }
}
