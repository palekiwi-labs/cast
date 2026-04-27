use anyhow::Result;

use crate::config::Config;
use crate::dev::run::RunOpts;
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::user::ResolvedUser;

/// A harness encapsulates everything that is specific to a particular program
/// run inside the dev container (e.g. OpenCode, ClaudeCode).
///
/// Generic docker run arguments (security, resource limits, workspace mount,
/// shadow mounts, etc.) are assembled by the caller. The harness is responsible
/// only for the program-specific layer on top.
pub trait Harness {
    /// Short identifier used in container names and CLI subcommands (e.g. `"opencode"`).
    fn name(&self) -> &str;

    /// Resolve the Docker image tag that should be used for this harness.
    fn image_tag(&self, config: &Config) -> Result<String>;

    /// Ensure the harness image exists locally, building it if necessary.
    fn ensure_image(
        &self,
        docker: &DockerClient,
        config: &Config,
        user: &ResolvedUser,
        opts: BuildOptions,
    ) -> Result<()>;

    /// Return harness-specific `docker run` arguments (env vars, mounts, etc.)
    /// that are appended after the generic arguments.
    fn extra_run_args(&self, config: &Config, opts: &RunOpts) -> Result<Vec<String>>;

    /// Build the command vector that will be passed to `docker run` after all flags.
    fn command(&self, config: &Config, user: &ResolvedUser, extra_args: Vec<String>)
        -> Vec<String>;
}
