use anyhow::Result;
use clap::Subcommand;

use crate::config::Config;
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::nix;

#[derive(Subcommand)]
pub enum NixCommands {
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

pub fn handle_nix(cfg: &Config, command: Option<NixCommands>) -> Result<()> {
    match command {
        Some(NixCommands::Start) => {
            let docker = DockerClient;
            nix::ensure_running(&docker, cfg)?;
            Ok(())
        }
        Some(NixCommands::Stop) => {
            let docker = DockerClient;
            nix::stop(&docker, cfg)?;
            Ok(())
        }
        Some(NixCommands::BuildDaemon { force, no_cache }) => {
            let docker = DockerClient;
            let opts = BuildOptions { force, no_cache };
            nix::build(&docker, opts)?;
            Ok(())
        }
        None => {
            // No subcommand provided, print help for nix
            println!("Usage: ocx nix <COMMAND>");
            println!();
            println!("Commands:");
            println!("  start    Start the nix daemon container");
            println!("  stop     Stop the nix daemon container");
            println!("  build    Build the nix daemon image");
            Ok(())
        }
    }
}
