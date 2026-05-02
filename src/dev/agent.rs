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

    /// The fundamental binary command of the agent (e.g. `"opencode"` or `"pi"`).
    fn base_command(&self) -> &'static str;

    /// Build the command vector that will be passed to `docker run` after all flags.
    /// Default implementation handles the nested Nix develop wrapping logic.
    fn build_command(
        &self,
        config: &Config,
        opts: &RunOpts,
        extra_args: Vec<String>,
    ) -> Vec<String> {
        let mut cmd: Vec<String> = vec![self.base_command().to_string()];
        cmd.extend(extra_args);

        // Project flake (inner layer)
        if config.use_flake {
            let project_flake = if let Some(path) = &config.use_flake_path {
                Some(path.as_str())
            } else if opts.project_flake_present {
                Some(".")
            } else {
                None
            };

            if let Some(flake_ref) = project_flake {
                cmd = ["nix", "develop", flake_ref, "-c"]
                    .iter()
                    .map(|s| s.to_string())
                    .chain(cmd)
                    .collect();
            }
        }

        // Global flake (outer layer - always applies if present)
        if opts.user_flake_present {
            let global_flake = format!("/home/{}/.config/cast/nix", opts.user.username);
            cmd = ["nix", "develop", global_flake.as_str(), "-c"]
                .iter()
                .map(|s| s.to_string())
                .chain(cmd)
                .collect();
        }

        cmd
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
        fn base_command(&self) -> &'static str {
            "test"
        }
    }

    fn alice() -> ResolvedUser {
        ResolvedUser {
            username: "alice".to_string(),
            uid: 1000,
            gid: 1000,
        }
    }

    fn run_opts(user_flake_present: bool, project_flake_present: bool) -> RunOpts {
        RunOpts {
            workspace: ResolvedWorkspace {
                root: PathBuf::from("/work"),
                container_path: PathBuf::from("/work"),
            },
            user: alice(),
            port: 8080,
            host_home_dir: None,
            user_flake_present,
            project_flake_present,
        }
    }

    #[test]
    fn test_build_command_use_flake_false_no_global() {
        let mut config = Config::default();
        config.use_flake = false;
        config.use_flake_path = Some(".#my-shell".to_string());

        // Scenario 1: use_flake false, no global flake -> bare command
        let opts = run_opts(false, true);
        let cmd = TestAgent.build_command(&config, &opts, vec!["arg1".to_string()]);
        assert_eq!(cmd, vec!["test", "arg1"]);
    }

    #[test]
    fn test_build_command_use_flake_false_with_global() {
        let mut config = Config::default();
        config.use_flake = false;
        config.use_flake_path = Some(".#my-shell".to_string());

        // Scenario 2: use_flake false, global flake present -> wrapped ONLY in global flake
        let opts = run_opts(true, true);
        let cmd = TestAgent.build_command(&config, &opts, vec!["arg1".to_string()]);
        assert_eq!(
            cmd,
            vec![
                "nix",
                "develop",
                "/home/alice/.config/cast/nix",
                "-c",
                "test",
                "arg1"
            ]
        );
    }

    #[test]
    fn test_build_command_use_flake_true_no_global_no_project() {
        let mut config = Config::default();
        config.use_flake = true;
        config.use_flake_path = None;

        // Scenario 3: use_flake true, no global, no project flake -> bare command
        let opts = run_opts(false, false);
        let cmd = TestAgent.build_command(&config, &opts, vec!["arg1".to_string()]);
        assert_eq!(cmd, vec!["test", "arg1"]);
    }

    #[test]
    fn test_build_command_use_flake_true_no_global_with_project_auto() {
        let mut config = Config::default();
        config.use_flake = true;
        config.use_flake_path = None;

        // Scenario 4: use_flake true, no global, project flake present (auto-detect)
        let opts = run_opts(false, true);
        let cmd = TestAgent.build_command(&config, &opts, vec!["arg1".to_string()]);
        assert_eq!(cmd, vec!["nix", "develop", ".", "-c", "test", "arg1"]);
    }

    #[test]
    fn test_build_command_use_flake_true_no_global_with_project_path() {
        let mut config = Config::default();
        config.use_flake = true;
        config.use_flake_path = Some(".#shell".to_string());

        // Scenario 5: use_flake true, no global, explicit project flake path
        let opts = run_opts(false, false); // project_flake_present false to prove path overrides it
        let cmd = TestAgent.build_command(&config, &opts, vec!["arg1".to_string()]);
        assert_eq!(cmd, vec!["nix", "develop", ".#shell", "-c", "test", "arg1"]);
    }

    #[test]
    fn test_build_command_use_flake_true_with_global_and_project_auto() {
        let mut config = Config::default();
        config.use_flake = true;
        config.use_flake_path = None;

        // Scenario 6: use_flake true, global present, project auto-detected -> nested wrap
        let opts = run_opts(true, true);
        let cmd = TestAgent.build_command(&config, &opts, vec!["arg1".to_string()]);
        assert_eq!(
            cmd,
            vec![
                "nix",
                "develop",
                "/home/alice/.config/cast/nix",
                "-c",
                "nix",
                "develop",
                ".",
                "-c",
                "test",
                "arg1"
            ]
        );
    }

    #[test]
    fn test_build_command_use_flake_true_with_global_and_project_path() {
        let mut config = Config::default();
        config.use_flake = true;
        config.use_flake_path = Some(".#shell".to_string());

        // Scenario 7: use_flake true, global present, explicit project path -> nested wrap
        let opts = run_opts(true, false);
        let cmd = TestAgent.build_command(&config, &opts, vec!["arg1".to_string()]);
        assert_eq!(
            cmd,
            vec![
                "nix",
                "develop",
                "/home/alice/.config/cast/nix",
                "-c",
                "nix",
                "develop",
                ".#shell",
                "-c",
                "test",
                "arg1"
            ]
        );
    }
}
