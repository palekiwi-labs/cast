use anyhow::Result;
use clap::{Parser, Subcommand};

use super::{config, nix, port};
use crate::config::load_config;

/// ocx - a secure Docker wrapper for OpenCode
#[derive(Parser)]
#[command(name = "ocx")]
#[command(about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage OCX configuration
    Config {
        #[command(subcommand)]
        command: Option<config::ConfigCommands>,
    },
    /// Manage Nix daemon
    Nix {
        #[command(subcommand)]
        command: Option<nix::NixCommands>,
    },
    /// Print the port that the container will publish
    Port,
}

pub fn run(cli: Cli) -> Result<()> {
    // Load config once at startup for efficiency and consistency
    let cfg = load_config()?;

    match cli.command {
        Some(Commands::Config { command }) => config::handle_config(&cfg, command),
        Some(Commands::Nix { command }) => nix::handle_nix(&cfg, command),
        Some(Commands::Port) => port::handle_port(&cfg),
        None => {
            // No subcommand provided, print help
            Cli::parse_from(["ocx", "--help"]);
            Ok(())
        }
    }
}
