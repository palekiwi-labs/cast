pub mod config_dir;
pub mod env;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::Config;
use crate::dev::agent::Agent;
use crate::dev::run::RunOpts;
use crate::dev::utils;
use crate::dev::version::fetcher::GithubReleaseFetcher;
use crate::dev::version::{self, VersionResolver};
use crate::user::ResolvedUser;

/// Resolve the concrete opencode version based on config.
pub fn resolve_version(config: &Config) -> Result<String> {
    let requested = config
        .agent_versions
        .get("opencode")
        .map(|s| s.as_str())
        .unwrap_or("latest");
    let cache_path = version::cache::get_cache_path("opencode");
    let resolver = VersionResolver::new(cache_path, config.version_cache_ttl_hours);
    let fetcher = GithubReleaseFetcher {
        repo: "anomalyco/opencode",
    };
    resolver.resolve(requested, &fetcher)
}

/// Resolves the OPENCODE_CONFIG_DIR environment variable to an absolute host
/// path, expanding leading tildes if necessary, and filtering on path existence.
pub fn resolve_config_dir_env(
    env_val: Option<String>,
    home_dir: Option<&Path>,
) -> Result<Option<PathBuf>> {
    let path = match env_val {
        Some(v) => utils::expand_tilde(&v, home_dir),
        None => return Ok(None),
    };

    if path.exists() {
        Ok(Some(path))
    } else {
        anyhow::bail!(
            "OPENCODE_CONFIG_DIR path does not exist: {}",
            path.display()
        )
    }
}

/// Resolves the OPENCODE_CONFIG environment variable to an absolute host
/// path, expanding leading tildes if necessary, and filtering on file existence.
pub fn resolve_config_file_env(
    env_val: Option<String>,
    home_dir: Option<&Path>,
) -> Result<Option<PathBuf>> {
    let path = match env_val {
        Some(v) => utils::expand_tilde(&v, home_dir),
        None => return Ok(None),
    };

    if path.is_file() {
        Ok(Some(path))
    } else {
        anyhow::bail!("OPENCODE_CONFIG path is not a file: {}", path.display())
    }
}

/// The OpenCode agent — runs the `opencode` program inside the dev container.
pub struct OpenCode;

impl Agent for OpenCode {
    fn name(&self) -> &'static str {
        "opencode"
    }

    fn dockerfile(&self) -> &'static str {
        include_str!("../../../assets/Dockerfile.dev.opencode")
    }

    fn resolve_version(&self, config: &Config) -> Result<String> {
        resolve_version(config)
    }

    fn prepare_host(&self, _config: &Config, _opts: &RunOpts) -> Result<()> {
        let base = dirs::config_dir().context("Failed to resolve user config directory")?;
        config_dir::ensure_config_dir(&base)?;
        Ok(())
    }

    fn base_command<'a>(&self, config: &'a Config) -> &'a [String] {
        &config.opencode_command
    }

    fn extra_run_args(
        &self,
        config: &Config,
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
        )?;
        if let Some(config_dir_env) = &opencode_config_dir_env {
            args.extend([
                "-v".to_string(),
                format!("{}:/opencode-config-dir:ro", config_dir_env.display()),
                "-e".to_string(),
                "OPENCODE_CONFIG_DIR=/opencode-config-dir".to_string(),
            ]);
        }

        // OPENCODE_CONFIG special case: bind-mount file with container path rewrite.
        let opencode_config_env = resolve_config_file_env(
            env.get("OPENCODE_CONFIG").cloned(),
            opts.host_home_dir.as_deref(),
        )?;
        if let Some(config_file_env) = &opencode_config_env {
            args.extend([
                "-v".to_string(),
                format!("{}:/opencode.json:ro", config_file_env.display()),
                "-e".to_string(),
                "OPENCODE_CONFIG=/opencode.json".to_string(),
            ]);
        }

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

        // Persistent data volumes (~/.cache and ~/.local).
        args.extend(build_data_volume_args(config, &opts.user));

        Ok(args)
    }
}

/// Persistent data volumes for OpenCode: `<namespace>-opencode-cache` and `<namespace>-opencode-local`.
fn build_data_volume_args(cfg: &Config, user: &ResolvedUser) -> Vec<String> {
    let namespace = &cfg.volumes_namespace;
    let username = &user.username;
    vec![
        "-v".to_string(),
        format!("{}-opencode-cache:/home/{}/.cache:rw", namespace, username),
        "-v".to_string(),
        format!("{}-opencode-local:/home/{}/.local:rw", namespace, username),
    ]
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
            user_flake_present: false,
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
            resolve_config_dir_env(Some("~/.config/my-opencode".to_string()), Some(home)).unwrap();
        assert_eq!(result, Some(target_dir));
    }

    #[test]
    fn test_resolve_config_dir_env_absolute() {
        let temp = tempfile::TempDir::new().unwrap();
        let target_dir = temp.path().join("absolute/path");
        std::fs::create_dir_all(&target_dir).unwrap();

        let result =
            resolve_config_dir_env(Some(target_dir.to_string_lossy().to_string()), None).unwrap();
        assert_eq!(result, Some(target_dir));
    }

    #[test]
    fn test_resolve_config_dir_env_missing() {
        let result =
            resolve_config_dir_env(Some("/does/not/exist/anywhere/12345".to_string()), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_config_file_env_absolute() {
        let temp = tempfile::TempDir::new().unwrap();
        let target_file = temp.path().join("config.json");
        std::fs::write(&target_file, "{}").unwrap();

        let result =
            resolve_config_file_env(Some(target_file.to_string_lossy().to_string()), None).unwrap();
        assert_eq!(result, Some(target_file));
    }

    #[test]
    fn test_resolve_config_file_env_not_file() {
        let temp = tempfile::TempDir::new().unwrap();
        let target_dir = temp.path().join("not-a-file");
        std::fs::create_dir_all(&target_dir).unwrap();

        let result = resolve_config_file_env(Some(target_dir.to_string_lossy().to_string()), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_config_file_env_missing() {
        let result =
            resolve_config_file_env(Some("/does/not/exist/anywhere/12345".to_string()), None);
        assert!(result.is_err());
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
            assert!(
                !arg.contains("/.config/cast/nix"),
                "unexpected flake mount: {}",
                arg
            );
        }
    }

    #[test]
    fn test_extra_run_args_opencode_config_dir_env_unset() {
        let config = Config::default();
        let opts = basic_opts(PathBuf::from("/home/alice/project"));
        let env = HashMap::new();

        let args = OpenCode.extra_run_args(&config, &opts, &env).unwrap();

        for arg in &args {
            assert!(
                !arg.contains("/opencode-config-dir"),
                "unexpected env mount: {}",
                arg
            );
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
            user_flake_present: false,
        };

        let args = OpenCode.extra_run_args(&config, &opts, &env).unwrap();

        assert!(args.contains(&format!(
            "{}:/opencode-config-dir:ro",
            config_dir_env.display()
        )));
    }

    #[test]
    fn test_extra_run_args_opencode_config_env_set() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_file_path = temp_dir.path().join("config.json");
        std::fs::write(&config_file_path, "{}").unwrap();

        let mut env = HashMap::new();
        env.insert(
            "OPENCODE_CONFIG".to_string(),
            config_file_path.to_str().unwrap().to_string(),
        );

        let config = Config::default();
        let opts = basic_opts(PathBuf::from("/home/alice/project"));

        let args = OpenCode.extra_run_args(&config, &opts, &env).unwrap();

        assert!(args.contains(&format!("{}:/opencode.json:ro", config_file_path.display())));
        assert!(args.contains(&"OPENCODE_CONFIG=/opencode.json".to_string()));
    }

    #[test]
    fn test_extra_run_args_workspace_conflict_no_double_mount() {
        let config = Config::default();
        // workspace root == opencode config dir → no duplicate mount
        let workspace_root = dirs::config_dir().unwrap().join("opencode");

        let opts = RunOpts {
            workspace: ResolvedWorkspace {
                container_path: workspace_root.clone(),
                root: workspace_root,
            },
            user: alice(),
            port: 32768,
            host_home_dir: Some(PathBuf::from("/home/alice")),
            user_flake_present: false,
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

    #[test]
    fn test_extra_run_args_includes_opencode_data_volumes() {
        let config = Config::default(); // volumes_namespace = "cast"
        let opts = basic_opts(PathBuf::from("/home/alice/project"));
        let env = HashMap::new();

        let args = OpenCode.extra_run_args(&config, &opts, &env).unwrap();

        assert!(args.contains(&"cast-opencode-cache:/home/alice/.cache:rw".to_string()));
        assert!(args.contains(&"cast-opencode-local:/home/alice/.local:rw".to_string()));
    }

    #[test]
    fn test_image_tag_format() {
        assert_eq!(
            OpenCode.image_tag("1.4.7"),
            format!(
                "localhost/cast:{}-opencode-1.4.7",
                env!("CARGO_PKG_VERSION")
            )
        );
    }

    #[test]
    fn test_dockerfile_has_correct_base_image() {
        assert!(OpenCode.dockerfile().contains("FROM debian:trixie-slim"));
    }
}
