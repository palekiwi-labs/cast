use std::collections::HashMap;

use anyhow::Result;

use crate::config::Config;
use crate::dev::image;
use crate::dev::run::RunOpts;
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::user::ResolvedUser;

/// An agent encapsulates everything that is specific to a particular program
/// run inside the dev container (e.g. OpenCode, ClaudeCode).
///
/// Generic docker run arguments (security, resource limits, workspace mount,
/// shadow mounts, etc.) are assembled by the caller. The agent is responsible
/// only for the program-specific layer on top.
pub trait Agent {
    /// Short identifier used in container names and CLI subcommands (e.g. `"opencode"`).
    fn name(&self) -> &'static str;

    /// Get the embedded Dockerfile content for this agent.
    fn dockerfile(&self) -> &'static str;

    /// Resolve the concrete version based on config.
    fn resolve_version(&self, config: &Config) -> Result<String>;

    /// Resolve the Docker image tag that should be used for this agent given a version.
    fn image_tag(&self, version: &str) -> String {
        image::image_tag(self.name(), version)
    }

    /// Ensure the agent image exists locally, building it if necessary.
    fn ensure_image(
        &self,
        docker: &DockerClient,
        config: &Config,
        user: &ResolvedUser,
        version: &str,
        opts: BuildOptions,
    ) -> Result<()> {
        image::ensure_image(
            self.name(),
            self.dockerfile(),
            docker,
            config,
            user,
            version,
            opts,
        )
    }

    /// Return agent-specific `docker run` arguments (env vars, mounts, etc.)
    /// that are appended after the generic arguments.
    fn extra_run_args(
        &self,
        config: &Config,
        opts: &RunOpts,
        env: &HashMap<String, String>,
    ) -> Result<Vec<String>>;

    /// Perform host-side preparation (e.g. create directories) before the container
    /// runs. Default implementation is a no-op; agents override as needed.
    fn prepare_host(&self, _config: &Config, _opts: &RunOpts) -> Result<()> {
        Ok(())
    }

    /// The fundamental command vector from config (e.g. `&config.opencode_command`).
    fn base_command<'a>(&self, config: &'a Config) -> &'a [String];

    /// Hook to enable/disable the nix develop wrapper.
    fn use_nix_develop_wrapper(&self) -> bool {
        true
    }

    /// Build the command vector that will be passed to `docker run` after all flags.
    /// Default implementation handles the shared Nix develop wrapping logic.
    fn build_command(
        &self,
        config: &Config,
        opts: &RunOpts,
        extra_args: Vec<String>,
    ) -> Vec<String> {
        let base = self.base_command(config);
        let mut full_cmd = if self.use_nix_develop_wrapper() && opts.user_flake_present {
            let flake_dir = format!("/home/{}/.config/cast/nix", opts.user.username);
            let mut c = vec![
                "nix".to_string(),
                "develop".to_string(),
                flake_dir,
                "-c".to_string(),
            ];
            c.extend(base.iter().cloned());
            c
        } else {
            base.to_vec()
        };
        full_cmd.extend(extra_args);
        full_cmd
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dev::workspace::ResolvedWorkspace;
    use crate::user::ResolvedUser;
    use std::path::PathBuf;

    struct TestAgent;
    impl Agent for TestAgent {
        fn name(&self) -> &'static str {
            "test"
        }
        fn dockerfile(&self) -> &'static str {
            ""
        }
        fn resolve_version(&self, _config: &Config) -> Result<String> {
            Ok("1.0.0".to_string())
        }
        fn extra_run_args(
            &self,
            _config: &Config,
            _opts: &RunOpts,
            _env: &HashMap<String, String>,
        ) -> Result<Vec<String>> {
            Ok(vec![])
        }
        fn base_command<'a>(&self, config: &'a Config) -> &'a [String] {
            &config.opencode_command
        }
    }

    fn alice() -> ResolvedUser {
        ResolvedUser {
            username: "alice".to_string(),
            uid: 1000,
            gid: 1000,
        }
    }

    fn run_opts(user_flake_present: bool) -> RunOpts {
        RunOpts {
            workspace: ResolvedWorkspace {
                root: PathBuf::from("/work"),
                container_path: PathBuf::from("/work"),
            },
            user: alice(),
            port: 8080,
            host_home_dir: None,
            user_flake_present,
        }
    }

    #[test]
    fn test_build_command_no_flake() {
        let config = Config::default();
        let opts = run_opts(false);
        let cmd = TestAgent.build_command(&config, &opts, vec!["arg1".to_string()]);
        assert_eq!(cmd, vec!["opencode", "arg1"]);
    }

    #[test]
    fn test_build_command_with_flake() {
        let config = Config::default();
        let opts = run_opts(true);
        let cmd = TestAgent.build_command(&config, &opts, vec!["arg1".to_string()]);
        assert_eq!(
            cmd,
            vec![
                "nix",
                "develop",
                "/home/alice/.config/cast/nix",
                "-c",
                "opencode",
                "arg1"
            ]
        );
    }

    #[test]
    fn test_build_command_no_nix_wrapper() {
        struct NoNixAgent;
        impl Agent for NoNixAgent {
            fn name(&self) -> &'static str {
                "nonix"
            }
            fn dockerfile(&self) -> &'static str {
                ""
            }
            fn resolve_version(&self, _config: &Config) -> Result<String> {
                Ok("1.0.0".to_string())
            }
            fn extra_run_args(
                &self,
                _config: &Config,
                _opts: &RunOpts,
                _env: &HashMap<String, String>,
            ) -> Result<Vec<String>> {
                Ok(vec![])
            }
            fn base_command<'a>(&self, config: &'a Config) -> &'a [String] {
                &config.pi_command
            }
            fn use_nix_develop_wrapper(&self) -> bool {
                false
            }
        }

        let config = Config::default();
        let opts = run_opts(true);
        let cmd = NoNixAgent.build_command(&config, &opts, vec![]);
        assert_eq!(cmd, vec!["pi"]);
    }
}
