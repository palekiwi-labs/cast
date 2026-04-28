use super::{build, config, nix_daemon, port};
use crate::config::load_config;
use crate::dev;
use crate::dev::opencode::OpenCode;
use anyhow::Result;
use clap::{Parser, Subcommand};

/// ocx - a secure Docker wrapper for OpenCode
#[derive(Parser)]
#[command(name = "ocx")]
#[command(about, long_about = None)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
#[command(subcommand_required = true)]
pub enum RunAgent {
    /// Start an interactive OpenCode session
    #[command(alias = "o", disable_help_flag = true)]
    Opencode {
        /// Extra arguments to pass to the opencode command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 0..)]
        extra_args: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum Commands {
    /// Build the Nix dev image (and optionally the daemon base image)
    Build {
        #[arg(long)]
        base: bool,
        #[arg(short, long)]
        force: bool,
        #[arg(long)]
        no_cache: bool,
    },
    /// Manage OCX configuration
    Config {
        #[command(subcommand)]
        command: Option<config::ConfigCommands>,
    },
    /// Manage Nix daemon
    #[command(name = "nix-daemon", arg_required_else_help = true)]
    NixDaemon {
        #[command(subcommand)]
        command: nix_daemon::NixDaemonCommands,
    },
    /// Print the port that the container will publish
    Port,
    /// Run an agent
    Run {
        #[command(subcommand)]
        agent: RunAgent,
    },
    /// Drop into an interactive shell in the dev container
    Shell,
}

pub fn run(cli: Cli) -> Result<()> {
    // Load config once at startup for efficiency and consistency
    let cfg = load_config()?;

    match cli.command {
        Some(Commands::Build {
            base,
            force,
            no_cache,
        }) => build::handle_build(&cfg, base, force, no_cache),
        Some(Commands::Config { command }) => config::handle_config(&cfg, command),
        Some(Commands::NixDaemon { command }) => nix_daemon::handle_nix_daemon(&cfg, command),
        Some(Commands::Port) => port::handle_port(&cfg),
        Some(Commands::Run {
            agent: RunAgent::Opencode { extra_args },
        }) => dev::run_agent(&OpenCode, &cfg, extra_args),
        Some(Commands::Shell) => dev::shell(&OpenCode, &cfg),
        None => unreachable!("Clap should handle required subcommands"),
    }
}
