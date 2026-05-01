use anyhow::{Context, Result};

use crate::config::Config;
use crate::dev::agent::Agent;
use crate::dev::run::RunOpts;
use crate::dev::version::fetcher::GithubReleaseFetcher;
use crate::dev::version::{self, VersionResolver};
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::user::ResolvedUser;
use std::collections::HashMap;

pub mod cmd;
pub mod config_dir;
pub mod env;
pub mod image;

/// Resolve the concrete pi version based on config.
pub fn resolve_version(config: &Config) -> Result<String> {
    let requested = config
        .agent_versions
        .get("pi")
        .map(|s| s.as_str())
        .unwrap_or("latest");
    let cache_path = version::cache::get_cache_path("pi");
    let resolver = VersionResolver::new(cache_path, config.version_cache_ttl_hours);
    let fetcher = GithubReleaseFetcher {
        repo: "badlogic/pi-mono",
    };
    resolver.resolve(requested, &fetcher)
}

pub struct Pi;

impl Agent for Pi {
    fn name(&self) -> &str {
        "pi"
    }

    fn image_tag(&self, config: &Config) -> Result<String> {
        let version = resolve_version(config)?;
        Ok(image::get_image_tag(&version))
    }

    fn ensure_image(
        &self,
        docker: &DockerClient,
        config: &Config,
        user: &ResolvedUser,
        opts: BuildOptions,
    ) -> Result<()> {
        let version = resolve_version(config)?;
        image::ensure_dev_image(docker, config, user, &version, opts)
    }

    fn prepare_host(&self, _config: &Config, _opts: &RunOpts) -> Result<()> {
        let base = dirs::config_dir().context("Failed to resolve user config directory")?;
        config_dir::ensure_config_dir(&base)?;
        Ok(())
    }

    fn extra_run_args(
        &self,
        config: &Config,
        opts: &RunOpts,
        env: &HashMap<String, String>,
    ) -> Result<Vec<String>> {
        // LLM API keys + PI_* env vars present on the host.
        let mut args = env::build_passthrough_env_args(env);

        // Pi config directory bind mount.
        let base = dirs::config_dir().context("Failed to resolve user config directory")?;
        let pi_config_host_dir = config_dir::get_config_dir(&base);

        args.extend([
            "-v".to_string(),
            format!(
                "{}:/home/{}/.pi:rw",
                pi_config_host_dir.display(),
                opts.user.username
            ),
            "-e".to_string(),
            format!("PI_CODING_AGENT_DIR=/home/{}/.pi", opts.user.username),
        ]);

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

    fn command(&self, config: &Config, opts: &RunOpts, extra_args: Vec<String>) -> Vec<String> {
        let mut command = cmd::resolve_pi_command(config, &opts.user, opts.user_flake_present);
        command.extend(extra_args);
        command
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
            user_flake_present: false,
        };
        let mut env = HashMap::new();
        env.insert("ANTHROPIC_API_KEY".to_string(), "sk-123".to_string());

        let pi = Pi;
        let args = pi.extra_run_args(&config, &run_opts, &env).unwrap();

        assert!(args.contains(&"-e".to_string()));
        assert!(args.contains(&"ANTHROPIC_API_KEY".to_string()));
        assert!(args.contains(&"PI_CODING_AGENT_DIR=/home/testuser/.pi".to_string()));
    }
}
