use anyhow::Result;
use clap::Subcommand;

use crate::config::Config;
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::nix_daemon;

#[derive(Subcommand)]
pub enum NixDaemonCommands {
    /// Build the nix daemon image
    #[command(name = "build")]
    Build {
        /// Force rebuild even if image exists
        #[arg(long)]
        force: bool,

        /// Do not use cache when building
        #[arg(long)]
        no_cache: bool,
    },
    /// Drop into an interactive shell in the nix daemon container
    Shell,
    /// Start the nix daemon container
    Start,
    /// Stop the nix daemon container
    Stop,
}

pub fn handle_nix_daemon(cfg: &Config, command: NixDaemonCommands) -> Result<()> {
    match command {
        NixDaemonCommands::Build { force, no_cache } => {
            let docker = DockerClient;
            let opts = BuildOptions { force, no_cache };
            nix_daemon::build(&docker, opts)?;
            Ok(())
        }
        NixDaemonCommands::Shell => {
            let docker = DockerClient;
            let status = nix_daemon::shell(&docker, cfg)?;
            std::process::exit(status.code().unwrap_or(1));
        }
        NixDaemonCommands::Start => {
            let docker = DockerClient;
            nix_daemon::ensure_running(&docker, cfg)?;
            Ok(())
        }
        NixDaemonCommands::Stop => {
            let docker = DockerClient;
            nix_daemon::stop(&docker, cfg)?;
            Ok(())
        }
    }
}
