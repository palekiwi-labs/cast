use super::{build, config, nix, opencode, port};
use crate::config::load_config;
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
    /// Manage Nix daemon
    Nix {
        #[command(subcommand)]
        command: Option<nix::NixCommands>,
    },
    /// Start an interactive OpenCode session
    #[command(
        alias = "o",
        disable_help_flag = true,
    )]
    Opencode {
        /// Extra arguments to pass to the opencode command
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            num_args = 0..
        )]
        extra_args: Vec<String>,
    },
    /// Print the port that the container will publish
    Port,
    /// Build the Nix dev image (and optionally the daemon base image)
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
        Some(Commands::Opencode { extra_args }) => opencode::handle_opencode(&cfg, extra_args),
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
