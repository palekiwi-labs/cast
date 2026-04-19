use anyhow::Result;
use clap::{Parser, Subcommand};

use super::{build, config, nix, port};
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
    /// Build both the Nix daemon and Nix dev images
    Build {
        #[arg(long)]
        base: bool,
        #[arg(short, long)]
        force: bool,
        #[arg(long)]
        no_cache: bool,
    },
}

pub fn run(cli: Cli) -> Result<()> {
    // Load config once at startup for efficiency and consistency
    let cfg = load_config()?;

    match cli.command {
        Some(Commands::Config { command }) => config::handle_config(&cfg, command),
        Some(Commands::Nix { command }) => nix::handle_nix(&cfg, command),
        Some(Commands::Port) => port::handle_port(&cfg),
        Some(Commands::Build {
            base,
            force,
            no_cache,
        }) => build::handle_build(&cfg, base, force, no_cache),
        None => {
            // No subcommand provided, print help
            Cli::parse_from(["ocx", "--help"]);
            Ok(())
        }
    }
}
