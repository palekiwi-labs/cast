pub mod config_dir;
pub mod env;

use std::collections::HashMap;

use anyhow::{Context, Result};

use crate::config::Config;
use crate::dev::agent::Agent;
use crate::dev::run::RunOpts;
use crate::dev::version::fetcher::NpmRegistryFetcher;
use crate::dev::version::{self, VersionResolver};
use crate::user::ResolvedUser;

/// Resolve the concrete claudecode version based on config.
pub fn resolve_version(config: &Config) -> Result<String> {
    let requested = config
        .agent_versions
        .get("claudecode")
        .map(|s| s.as_str())
        .unwrap_or("latest");
    let cache_path = version::cache::get_cache_path("claudecode");
    let resolver = VersionResolver::new(cache_path, config.version_cache_ttl_hours);
    let fetcher = NpmRegistryFetcher {
        package: "@anthropic-ai/claude-code",
    };
    resolver.resolve(requested, &fetcher)
}

/// The ClaudeCode agent — runs the `claude` program inside the dev container.
pub struct ClaudeCode;

impl Agent for ClaudeCode {
    fn name(&self) -> &'static str {
        "claudecode"
    }

    fn dockerfile(&self) -> &'static str {
        include_str!("../../../assets/Dockerfile.dev.claudecode")
    }

    fn resolve_version(&self, config: &Config) -> Result<String> {
        resolve_version(config)
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

    fn extra_run_args(
        &self,
        config: &Config,
        opts: &RunOpts,
        env: &HashMap<String, String>,
    ) -> Result<Vec<String>> {
        // LLM API keys + Claude Code env vars present on the host.
        let mut args = env::build_passthrough_env_args(env);

        // Claude Code config directory bind mount (~/.claude).
        let home = opts
            .host_home_dir
            .as_deref()
            .context("Failed to resolve user home directory")?;
        let claude_config_dir = config_dir::get_config_dir(home);
        args.extend([
            "-v".to_string(),
            format!(
                "{}:/home/{}/.claude:rw",
                claude_config_dir.display(),
                opts.user.username
            ),
        ]);

        // Claude Code global config file bind mount (~/.claude.json).
        let claude_config_file = config_dir::get_config_file(home);
        args.extend([
            "-v".to_string(),
            format!(
                "{}:/home/{}/.claude.json:rw",
                claude_config_file.display(),
                opts.user.username
            ),
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
}

/// Persistent data volumes for ClaudeCode: `<ns>-claudecode-cache` and `<ns>-claudecode-local`.
fn build_data_volume_args(cfg: &Config, user: &ResolvedUser) -> Vec<String> {
    let namespace = &cfg.volumes_namespace;
    let username = &user.username;
    vec![
        "-v".to_string(),
        format!(
            "{}-claudecode-cache:/home/{}/.cache:rw",
            namespace, username
        ),
        "-v".to_string(),
        format!(
            "{}-claudecode-local:/home/{}/.local:rw",
            namespace, username
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::dev::run::RunOpts;
    use crate::dev::workspace::ResolvedWorkspace;
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
            user_flake_present: false,
            project_flake_present: false,
        }
    }

    #[test]
    fn test_extra_run_args_includes_claude_config_mount() {
        let config = Config::default();
        let opts = basic_opts(PathBuf::from("/tmp/workspace"));
        let env = HashMap::new();

        let args = ClaudeCode.extra_run_args(&config, &opts, &env).unwrap();

        assert!(
            args.contains(&"/home/testuser/.claude:/home/testuser/.claude:rw".to_string()),
            "expected claude config bind mount in args: {:?}",
            args
        );
    }

    #[test]
    fn test_extra_run_args_includes_claude_json_mount() {
        let config = Config::default();
        let opts = basic_opts(PathBuf::from("/tmp/workspace"));
        let env = HashMap::new();

        let args = ClaudeCode.extra_run_args(&config, &opts, &env).unwrap();

        assert!(
            args.contains(
                &"/home/testuser/.claude.json:/home/testuser/.claude.json:rw".to_string()
            ),
            "expected claude.json bind mount in args: {:?}",
            args
        );
    }

    #[test]
    fn test_extra_run_args_includes_data_volumes() {
        let config = Config::default(); // volumes_namespace = "cast"
        let opts = basic_opts(PathBuf::from("/tmp/workspace"));
        let env = HashMap::new();

        let args = ClaudeCode.extra_run_args(&config, &opts, &env).unwrap();

        assert!(args.contains(&"cast-claudecode-cache:/home/testuser/.cache:rw".to_string()));
        assert!(args.contains(&"cast-claudecode-local:/home/testuser/.local:rw".to_string()));
    }

    #[test]
    fn test_extra_run_args_user_flake_absent() {
        let config = Config::default();
        let opts = basic_opts(PathBuf::from("/tmp/workspace"));
        let env = HashMap::new();

        let args = ClaudeCode.extra_run_args(&config, &opts, &env).unwrap();

        for arg in &args {
            assert!(
                !arg.contains("/.config/cast/nix"),
                "unexpected flake mount: {}",
                arg
            );
        }
    }

    #[test]
    fn test_extra_run_args_passthrough_env() {
        let config = Config::default();
        let opts = basic_opts(PathBuf::from("/tmp/workspace"));
        let mut env = HashMap::new();
        env.insert("ANTHROPIC_API_KEY".to_string(), "sk-abc".to_string());

        let args = ClaudeCode.extra_run_args(&config, &opts, &env).unwrap();

        assert!(args.contains(&"-e".to_string()));
        assert!(args.contains(&"ANTHROPIC_API_KEY".to_string()));
    }

    #[test]
    fn test_image_tag_format() {
        assert_eq!(
            ClaudeCode.image_tag("1.2.3"),
            format!(
                "localhost/cast:{}-claudecode-1.2.3",
                env!("CARGO_PKG_VERSION")
            )
        );
    }

    #[test]
    fn test_dockerfile_has_correct_base_image() {
        assert!(ClaudeCode.dockerfile().contains("FROM debian:trixie-slim"));
    }

    #[test]
    fn test_dockerfile_copies_node_from_official_image() {
        assert!(
            ClaudeCode
                .dockerfile()
                .contains("COPY --from=node:lts-trixie-slim /usr/local /usr/local")
        );
    }
}
