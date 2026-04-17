use anyhow::Result;
use clap::Subcommand;

use crate::config::Config;
use crate::nix::{self, DockerCliClient};

#[derive(Subcommand)]
pub enum NixCommands {
    /// Start the nix daemon container
    Start,
}

pub fn handle_nix(cfg: &Config, command: Option<NixCommands>) -> Result<()> {
    match command {
        Some(NixCommands::Start) => {
            let docker = DockerCliClient;
            nix::ensure_running(&docker, cfg)?;
            Ok(())
        }
        None => {
            // No subcommand provided, print help for nix
            println!("Usage: ocx nix <COMMAND>");
            println!();
            println!("Commands:");
            println!("  start    Start the nix daemon container");
            Ok(())
        }
    }
}
