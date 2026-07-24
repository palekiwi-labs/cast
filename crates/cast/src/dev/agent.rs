use std::collections::HashMap;

use anyhow::Result;

use crate::config::Config;
use crate::dev::build_command;
use crate::dev::run::RunOpts;

/// An agent encapsulates everything that is specific to a particular program
/// run inside the dev container (e.g. OpenCode, ClaudeCode).
///
/// Generic docker run arguments (security, resource limits, workspace mount,
/// shadow mounts, etc.) are assembled by the caller. The agent is responsible
/// only for the program-specific layer on top.
///
/// The dev image itself is shared and harness-free (see [`crate::dev::image`]);
/// agents no longer contribute a Dockerfile or a version — harnesses are
/// provided by the selected global Nix devShell.
pub trait Agent {
    /// Short identifier used in container names and CLI subcommands (e.g. `"opencode"`).
    fn name(&self) -> &'static str;

    /// Return agent-specific `docker run` arguments (env vars, mounts, etc.)
    /// that are appended after the generic arguments.
    fn extra_run_args(
        &self,
        config: &Config,
        opts: &RunOpts,
        env: &HashMap<String, String>,
    ) -> Result<Vec<String>>;

    /// Return only the config-directory bind mounts for this agent.
    ///
    /// Called for every known agent to compose the union of all config mounts
    /// (universal mounts are the unconditional default).
    fn config_mount_args(&self, _config: &Config, _opts: &RunOpts) -> Result<Vec<String>> {
        Ok(vec![])
    }

    /// Return the agent's environment-variable passthrough args (`-e VAR` pairs).
    ///
    /// Called for the launched agent only (subprocess agents inherit the
    /// container env).
    fn env_passthrough_args(&self, _env: &HashMap<String, String>) -> Vec<String> {
        Vec::new()
    }

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
        build_command::build_command(config, opts, self.base_command(), self.name(), extra_args)
    }
}
