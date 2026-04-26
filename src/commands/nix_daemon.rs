use anyhow::Result;
use clap::Subcommand;

use crate::config::Config;
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::nix_daemon;

#[derive(Subcommand)]
pub enum NixDaemonCommands {
    /// Start the nix daemon container
    Start,
    /// Stop the nix daemon container
    Stop,
    /// Build the nix daemon image
    #[command(name = "build")]
    BuildDaemon {
        /// Force rebuild even if image exists
        #[arg(long)]
        force: bool,

        /// Do not use cache when building
        #[arg(long)]
        no_cache: bool,
    },
}

pub fn handle_nix_daemon(cfg: &Config, command: Option<NixDaemonCommands>) -> Result<()> {
    match command {
        Some(NixDaemonCommands::Start) => {
            let docker = DockerClient;
            nix_daemon::ensure_running(&docker, cfg)?;
            Ok(())
        }
        Some(NixDaemonCommands::Stop) => {
            let docker = DockerClient;
            nix_daemon::stop(&docker, cfg)?;
            Ok(())
        }
        Some(NixDaemonCommands::BuildDaemon { force, no_cache }) => {
            let docker = DockerClient;
            let opts = BuildOptions { force, no_cache };
            nix_daemon::build(&docker, opts)?;
            Ok(())
        }
        None => {
            // No subcommand provided, print help for nix-daemon
            println!("Usage: ocx nix-daemon <COMMAND>");
            println!();
            println!("Commands:");
            println!("  start    Start the nix daemon container");
            println!("  stop     Stop the nix daemon container");
            println!("  build    Build the nix daemon image");
            Ok(())
        }
    }
}
