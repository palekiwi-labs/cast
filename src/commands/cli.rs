use super::{config, nix_daemon, port};
use crate::config::load_config;
use crate::dev;
use crate::dev::agent::Agent;
use crate::dev::opencode::OpenCode;
use crate::dev::pi::Pi;
use anyhow::Result;
use clap::{Parser, Subcommand};

/// cast - coding agent sandbox tool
#[derive(Parser)]
#[command(name = "cast")]
#[command(about, long_about = None, version)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
#[command(subcommand_required = true)]
pub enum BuildAgent {
    /// Build the agent's Docker image
    Opencode {
        /// Also build the Nix daemon base image
        #[arg(long)]
        base: bool,
        /// Force rebuild even if image already exists
        #[arg(short, long)]
        force: bool,
        /// Do not use Docker cache
        #[arg(long)]
        no_cache: bool,
    },
    /// Build the Pi agent's Docker image
    Pi {
        /// Also build the Nix daemon base image
        #[arg(long)]
        base: bool,
        /// Force rebuild even if image already exists
        #[arg(short, long)]
        force: bool,
        /// Do not use Docker cache
        #[arg(long)]
        no_cache: bool,
    },
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
    /// Start an interactive Pi session
    #[command(alias = "p", disable_help_flag = true)]
    Pi {
        /// Extra arguments to pass to the pi command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 0..)]
        extra_args: Vec<String>,
    },
}

#[derive(Subcommand)]
#[command(subcommand_required = true)]
pub enum ShellAgent {
    /// Drop into an interactive shell in the OpenCode container
    Opencode,
    /// Drop into an interactive shell in the Pi container
    Pi,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Build an agent's image
    Build {
        #[command(subcommand)]
        agent: BuildAgent,
    },
    /// Manage cast configuration
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
    Port {
        #[command(subcommand)]
        agent: RunAgent,
    },
    /// Run an agent
    Run {
        #[command(subcommand)]
        agent: RunAgent,
    },
    /// Drop into an interactive shell in an agent's container
    Shell {
        #[command(subcommand)]
        agent: ShellAgent,
    },
}

impl RunAgent {
    pub fn as_agent(&self) -> &'static dyn Agent {
        match self {
            RunAgent::Opencode { .. } => &OpenCode,
            RunAgent::Pi { .. } => &Pi,
        }
    }
}

pub fn run(cli: Cli) -> Result<()> {
    // Load config once at startup for efficiency and consistency
    let cfg = load_config()?;

    match cli.command {
        Some(Commands::Build {
            agent:
                BuildAgent::Opencode {
                    base,
                    force,
                    no_cache,
                },
        }) => dev::build_agent(&OpenCode, &cfg, base, force, no_cache),
        Some(Commands::Build {
            agent:
                BuildAgent::Pi {
                    base,
                    force,
                    no_cache,
                },
        }) => dev::build_agent(&Pi, &cfg, base, force, no_cache),
        Some(Commands::Config { command }) => config::handle_config(&cfg, command),
        Some(Commands::NixDaemon { command }) => nix_daemon::handle_nix_daemon(&cfg, command),
        Some(Commands::Port { agent }) => port::handle_port(&cfg, agent.as_agent()),
        Some(Commands::Run { agent }) => dev::run_agent(
            agent.as_agent(),
            &cfg,
            match &agent {
                RunAgent::Opencode { extra_args } => extra_args.clone(),
                RunAgent::Pi { extra_args } => extra_args.clone(),
            },
        ),
        Some(Commands::Shell {
            agent: ShellAgent::Opencode,
        }) => dev::shell(&OpenCode, &cfg),
        Some(Commands::Shell {
            agent: ShellAgent::Pi,
        }) => dev::shell(&Pi, &cfg),
        None => unreachable!("Clap should handle required subcommands"),
    }
}
