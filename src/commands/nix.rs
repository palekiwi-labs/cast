use anyhow::Result;
use clap::Subcommand;

use crate::config::Config;
use crate::nix::{self, DockerCliClient};

#[derive(Subcommand)]
pub enum NixCommands {
    /// Start the nix daemon container
    Start,
    /// Stop the nix daemon container
    Stop,
    /// Build the nix daemon image
    Build,
}

pub fn handle_nix(cfg: &Config, command: Option<NixCommands>) -> Result<()> {
    match command {
        Some(NixCommands::Start) => {
            let docker = DockerCliClient;
            nix::ensure_running(&docker, cfg)?;
            Ok(())
        }
        Some(NixCommands::Stop) => {
            let docker = DockerCliClient;
            nix::stop(&docker, cfg)?;
            Ok(())
        }
        Some(NixCommands::Build) => {
            let docker = DockerCliClient;
            nix::build(&docker)?;
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
