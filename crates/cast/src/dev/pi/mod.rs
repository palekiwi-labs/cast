use anyhow::{Context, Result};

use crate::config::Config;
use crate::dev::agent::Agent;
use crate::dev::run::RunOpts;
use crate::user::ResolvedUser;
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

    fn extra_run_args(
        &self,
        config: &Config,
        opts: &RunOpts,
        env: &HashMap<String, String>,
    ) -> Result<Vec<String>> {
        // LLM API keys + PI_* env vars present on the host.
        let mut args = self.env_passthrough_args(env);

        // Pi config directory bind mount.
        args.extend(self.config_mount_args(config, opts)?);

        // User flake mount (~/.config/cast/nix).
        let user_flake_host_dir = opts
            .host_home_dir
            .as_ref()
            .filter(|h| h.join(".config/cast/nix/flake.nix").exists())
            .map(|h| h.join(".config/cast/nix"));
        if let Some(flake_dir) = &user_flake_host_dir {
            args.extend([
                "-v".to_string(),
                format!(
                    "{}:/home/{}/.config/cast/nix:rw",
                    flake_dir.display(),
                    opts.user.username
                ),
            ]);
        }

        // Persistent data volumes (~/.cache and ~/.local).
        args.extend(build_data_volume_args(config, &opts.user));

        Ok(args)
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

/// Persistent data volumes for Pi: <namespace>-pi-cache and <namespace>-pi-local.
fn build_data_volume_args(cfg: &Config, user: &ResolvedUser) -> Vec<String> {
    let namespace = &cfg.volumes_namespace;
    let username = &user.username;
    vec![
        "-v".to_string(),
        format!("{}-pi-cache:/home/{}/.cache:rw", namespace, username),
        "-v".to_string(),
        format!("{}-pi-local:/home/{}/.local:rw", namespace, username),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_extra_run_args() {
        let config = Config::default();
        let user = ResolvedUser {
            uid: 1000,
            gid: 1000,
            username: "testuser".to_string(),
        };
        let run_opts = RunOpts {
            user: user.clone(),
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
        let mut env = HashMap::new();
        env.insert("ANTHROPIC_API_KEY".to_string(), "sk-123".to_string());

        let pi = Pi;
        let args = pi.extra_run_args(&config, &run_opts, &env).unwrap();

        assert!(args.contains(&"-v".to_string()));
        assert!(args.contains(&"/home/testuser/.pi:/home/testuser/.pi:rw".to_string()));
        assert!(args.contains(&"-e".to_string()));
        assert!(args.contains(&"ANTHROPIC_API_KEY".to_string()));
    }
}
