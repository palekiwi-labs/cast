pub mod cmd;
pub mod config_dir;
pub mod env;
pub mod image;
pub mod version;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::Config;
use crate::dev::agent::Agent;
use crate::dev::opencode::version::github::GithubVersionFetcher;
use crate::dev::opencode::version::{get_cache_path, resolve_version as do_resolve_version};
use crate::dev::run::RunOpts;
use crate::dev::utils;
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::user::ResolvedUser;

/// Resolve the concrete opencode version based on config.
pub fn resolve_version(config: &Config) -> Result<String> {
    let cache_path = get_cache_path();
    do_resolve_version(
        &config.opencode_version,
        config.version_cache_ttl_hours,
        &cache_path,
        &GithubVersionFetcher,
    )
}

/// Resolves the OPENCODE_CONFIG_DIR environment variable to an absolute host
/// path, expanding leading tildes if necessary, and filtering on path existence.
pub fn resolve_config_dir_env(env_val: Option<String>, home_dir: Option<&Path>) -> Option<PathBuf> {
    env_val
        .map(|p| utils::expand_tilde(&p, home_dir))
        .filter(|p| p.exists())
}

/// The OpenCode agent — runs the `opencode` program inside the dev container.
pub struct OpenCode;

impl Agent for OpenCode {
    fn name(&self) -> &str {
        "opencode"
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
        _config: &Config,
        opts: &RunOpts,
        env: &HashMap<String, String>,
    ) -> Result<Vec<String>> {
        let mut args: Vec<String> = vec![];

        // LLM API keys + OPENCODE_* env vars present on the host.
        args.extend(env::build_passthrough_env_args(env));

        // OPENCODE_CONFIG_DIR special case: bind-mount with container path rewrite.
        let opencode_config_dir_env = resolve_config_dir_env(
            env.get("OPENCODE_CONFIG_DIR").cloned(),
            opts.host_home_dir.as_deref(),
        );
        if let Some(config_dir_env) = &opencode_config_dir_env {
            args.extend([
                "-v".to_string(),
                format!("{}:/opencode-config-dir:ro", config_dir_env.display()),
                "-e".to_string(),
                "OPENCODE_CONFIG_DIR=/opencode-config-dir".to_string(),
            ]);
        }

        // User flake mount (~/.config/ocx/nix).
        let user_flake_host_dir = opts
            .host_home_dir
            .as_ref()
            .filter(|h| h.join(".config/ocx/nix/flake.nix").exists())
            .map(|h| h.join(".config/ocx/nix"));
        if let Some(flake_dir) = &user_flake_host_dir {
            args.extend([
                "-v".to_string(),
                format!(
                    "{}:/home/{}/.config/ocx/nix:rw",
                    flake_dir.display(),
                    opts.user.username
                ),
            ]);
        }

        // OpenCode config directory bind mount.
        // Skip if the workspace root is the same as the config dir (workspace mount covers it).
        let base = dirs::config_dir().context("Failed to resolve user config directory")?;
        let opencode_config_dir = config_dir::get_config_dir(&base);
        if opencode_config_dir != opts.workspace.root {
            args.extend([
                "-v".to_string(),
                format!(
                    "{}:/home/{}/.config/opencode:rw",
                    opencode_config_dir.display(),
                    opts.user.username
                ),
            ]);
        }

        Ok(args)
    }

    fn command(&self, config: &Config, user: &ResolvedUser, extra_args: Vec<String>) -> Vec<String> {
        let user_flake_present = dirs::home_dir()
            .filter(|h| h.join(".config/ocx/nix/flake.nix").exists())
            .is_some();
        let mut command = cmd::resolve_opencode_command(config, user, user_flake_present);
        command.extend(extra_args);
        command
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dev::run::RunOpts;
    use crate::dev::workspace::ResolvedWorkspace;
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
        }
    }

    // --- resolve_config_dir_env ---

    #[test]
    fn test_resolve_config_dir_env_with_tilde() {
        let temp = tempfile::TempDir::new().unwrap();
        let home = temp.path();

        let target_dir = home.join(".config/my-opencode");
        std::fs::create_dir_all(&target_dir).unwrap();

        let result =
            resolve_config_dir_env(Some("~/.config/my-opencode".to_string()), Some(home));
        assert_eq!(result, Some(target_dir));
    }

    #[test]
    fn test_resolve_config_dir_env_absolute() {
        let temp = tempfile::TempDir::new().unwrap();
        let target_dir = temp.path().join("absolute/path");
        std::fs::create_dir_all(&target_dir).unwrap();

        let result =
            resolve_config_dir_env(Some(target_dir.to_string_lossy().to_string()), None);
        assert_eq!(result, Some(target_dir));
    }

    #[test]
    fn test_resolve_config_dir_env_missing() {
        let result = resolve_config_dir_env(Some("/does/not/exist/anywhere/12345".to_string()), None);
        assert_eq!(result, None);
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
    fn test_extra_run_args_user_flake_absent() {
        let config = Config::default();
        let opts = basic_opts(PathBuf::from("/home/alice/project"));
        let env = HashMap::new();

        let args = OpenCode.extra_run_args(&config, &opts, &env).unwrap();

        for arg in &args {
            assert!(!arg.contains("/.config/ocx/nix"), "unexpected flake mount: {}", arg);
        }
    }

    #[test]
    fn test_extra_run_args_opencode_config_dir_env_unset() {
        let config = Config::default();
        let opts = basic_opts(PathBuf::from("/home/alice/project"));
        let env = HashMap::new();

        let args = OpenCode.extra_run_args(&config, &opts, &env).unwrap();

        for arg in &args {
            assert!(!arg.contains("/opencode-config-dir"), "unexpected env mount: {}", arg);
        }
    }

    #[test]
    fn test_extra_run_args_opencode_config_dir_env_set() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_dir_env = temp_dir.path().to_path_buf();
        let mut env = HashMap::new();
        env.insert(
            "OPENCODE_CONFIG_DIR".to_string(),
            config_dir_env.to_str().unwrap().to_string(),
        );

        let config = Config::default();
        let opts = basic_opts(PathBuf::from("/home/alice/project"));

        let args = OpenCode.extra_run_args(&config, &opts, &env).unwrap();

        assert!(args.contains(&format!(
            "{}:/opencode-config-dir:ro",
            config_dir_env.display()
        )));
        assert!(args.contains(&"OPENCODE_CONFIG_DIR=/opencode-config-dir".to_string()));
    }

    #[test]
    fn test_extra_run_args_opencode_config_dir_env_tilde() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let home_dir = temp_dir.path().to_path_buf();
        let config_dir_env = home_dir.join(".config/opencode-custom");
        std::fs::create_dir_all(&config_dir_env).unwrap();

        let mut env = HashMap::new();
        env.insert(
            "OPENCODE_CONFIG_DIR".to_string(),
            "~/.config/opencode-custom".to_string(),
        );

        let config = Config::default();
        let opts = RunOpts {
            workspace: ResolvedWorkspace {
                root: PathBuf::from("/home/alice/project"),
                container_path: PathBuf::from("/home/alice/project"),
            },
            user: alice(),
            port: 32768,
            host_home_dir: Some(home_dir),
        };

        let args = OpenCode.extra_run_args(&config, &opts, &env).unwrap();

        assert!(args.contains(&format!(
            "{}:/opencode-config-dir:ro",
            config_dir_env.display()
        )));
    }

    #[test]
    fn test_extra_run_args_workspace_conflict_no_double_mount() {
        let config = Config::default();
        // workspace root == opencode config dir → no duplicate mount
        let workspace_root = dirs::config_dir().unwrap().join("opencode");
        std::fs::create_dir_all(&workspace_root).unwrap();

        let opts = RunOpts {
            workspace: ResolvedWorkspace {
                container_path: workspace_root.clone(),
                root: workspace_root,
            },
            user: alice(),
            port: 32768,
            host_home_dir: Some(PathBuf::from("/home/alice")),
        };
        let env = HashMap::new();

        let args = OpenCode.extra_run_args(&config, &opts, &env).unwrap();

        // The opencode config dir mount must not appear (workspace covers it).
        let mount_count = args
            .iter()
            .filter(|a| a.contains("/.config/opencode:rw"))
            .count();
        assert_eq!(mount_count, 0);
    }
}
