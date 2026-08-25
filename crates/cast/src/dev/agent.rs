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
/// harnesses are provided by the configured global Nix devShell.
pub trait Agent {
    /// Short identifier used in container names and CLI subcommands (e.g. `"opencode"`).
    fn name(&self) -> &'static str;

    /// Return only the config-directory bind mounts for this agent.
    ///
    /// Called for every known agent to compose the union of all config mounts
    /// (universal mounts are the unconditional default).
    fn config_mount_args(&self, _config: &Config, _opts: &RunOpts) -> Result<Vec<String>> {
        Ok(vec![])
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
    fn build_command(&self, config: &Config, extra_args: Vec<String>) -> Vec<String> {
        build_command::build_command(config, self.base_command(), extra_args)
    }
}
