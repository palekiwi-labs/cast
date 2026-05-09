use std::process::ExitCode;

use anyhow::Result;
use clap::Subcommand;

use crate::config::ApprovedConfig;
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

pub fn handle_nix_daemon(cfg: &ApprovedConfig, command: NixDaemonCommands) -> Result<ExitCode> {
    match command {
        NixDaemonCommands::Build { force, no_cache } => {
            let docker = DockerClient;
            let opts = BuildOptions { force, no_cache };
            nix_daemon::build(&docker, opts)?;
            Ok(ExitCode::SUCCESS)
        }
        NixDaemonCommands::Shell => {
            let docker = DockerClient;
            let status = nix_daemon::shell(&docker, cfg)?;
            Ok(crate::commands::cli::to_exit_code(status))
        }
        NixDaemonCommands::Start => {
            let docker = DockerClient;
            nix_daemon::ensure_running(&docker, cfg)?;
            Ok(ExitCode::SUCCESS)
        }
        NixDaemonCommands::Stop => {
            let docker = DockerClient;
            nix_daemon::stop(&docker, cfg)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}
