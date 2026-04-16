pub mod config;
pub mod port;

use anyhow::Result;
use clap::{Parser, Subcommand};

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
    /// Print the port that the container will publish
    Port,
}

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Some(Commands::Config { command }) => config::handle_config(command),
        Some(Commands::Port) => port::handle_port(),
        None => {
            // No subcommand provided, print help
            Cli::parse_from(["ocx", "--help"]);
            Ok(())
        }
    }
}
