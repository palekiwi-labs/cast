use std::process::{ExitCode, ExitStatus};

use super::{config, nix_daemon, port};
use crate::config::{load_config, ApprovedConfig, Config};
use crate::dev;
use crate::dev::agent::Agent;
use crate::dev::claudecode::ClaudeCode;
use crate::dev::opencode::OpenCode;
use crate::dev::pi::Pi;
use crate::dev::run::{PublishPort, RunMode, SessionFlags};
use crate::dev::workspace::get_workspace;
use crate::logging::{generate_invocation_id, init_file_logger};
use crate::user::get_user;
use anyhow::{Context as _, Result};
use clap::{Parser, Subcommand};
use tracing::info_span;

/// Flags that control the execution mode of `cast run`.
#[derive(clap::Args, Clone, Debug)]
pub struct RunFlags {
    /// Run without a TTY (for CI, systemd, and piped output)
    #[arg(long)]
    pub headless: bool,

    /// Override the container name (default: auto-generated)
    #[arg(long)]
    pub name: Option<String>,

    /// Publish the container's port to the host.
    /// Without a value, uses the agent's deterministically calculated port.
    /// With a value, uses that specific host port (e.g. --publish 8080).
    #[arg(short = 'p', long, num_args = 0..=1, default_missing_value = "auto", value_name = "PORT")]
    pub publish: Option<PublishPort>,
}

/// cast - coding agent sandbox tool
#[derive(Parser)]
#[command(name = "cast")]
#[command(about, long_about = None, version)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Helper to verify configuration approval and return an ApprovedConfig.
fn verify_config(cfg: Config) -> Result<ApprovedConfig> {
    let user = get_user()?;
    let workspace = get_workspace(&user.username)?;
    crate::config::check_approved(cfg, &workspace.root)
}

pub fn run(cli: Cli) -> Result<ExitCode> {
    // Load config once at startup for efficiency and consistency
    let cfg = load_config()?;

    // Initialize file logger
    init_file_logger()?;

    let invocation_id = generate_invocation_id();
    let root = info_span!("cast", id = %invocation_id, pid = std::process::id());
    let _root_guard = root.enter();

    match cli.command {
        Some(Commands::Build {
            agent:
                BuildAgent::Opencode {
                    base,
                    force,
                    no_cache,
                },
        }) => {
            let approved = verify_config(cfg)?;
            dev::build_agent(&OpenCode, &approved, base, force, no_cache)?;
            Ok(ExitCode::SUCCESS)
        }
        Some(Commands::Build {
            agent:
                BuildAgent::Pi {
                    base,
                    force,
                    no_cache,
                },
        }) => {
            let approved = verify_config(cfg)?;
            dev::build_agent(&Pi, &approved, base, force, no_cache)?;
            Ok(ExitCode::SUCCESS)
        }
        Some(Commands::Build {
            agent:
                BuildAgent::Claudecode {
                    base,
                    force,
                    no_cache,
                },
        }) => {
            let approved = verify_config(cfg)?;
            dev::build_agent(&ClaudeCode, &approved, base, force, no_cache)?;
            Ok(ExitCode::SUCCESS)
        }
        Some(Commands::Config { command }) => config::handle_config(&cfg, command),
        Some(Commands::NixDaemon { command }) => {
            let approved = verify_config(cfg)?;
            nix_daemon::handle_nix_daemon(&approved, command)
        }
        Some(Commands::Port { agent }) => port::handle_port(&cfg, agent.as_agent()),
        Some(Commands::Run { flags, agent }) => {
            let approved = verify_config(cfg)?;
            let mode = if flags.headless {
                RunMode::Headless {
                    token: invocation_id.clone(),
                }
            } else {
                RunMode::Interactive
            };
            let session_flags = SessionFlags {
                mode,
                name: flags.name.clone(),
                publish: flags.publish.clone(),
            };
            let extra_args = match &agent {
                RunAgent::Opencode { extra_args } => extra_args.clone(),
                RunAgent::Pi { extra_args } => extra_args.clone(),
                RunAgent::Claudecode { extra_args } => extra_args.clone(),
            };
            let status = dev::run_agent(agent.as_agent(), &approved, session_flags, extra_args)?;
            Ok(to_exit_code(status))
        }
        Some(Commands::Shell { agent, raw }) => {
            let approved = verify_config(cfg)?;
            let status = dev::shell(agent.as_agent(), &approved, raw)?;
            Ok(to_exit_code(status))
        }
        #[cfg(feature = "mcp")]
        Some(Commands::Mcp { command }) => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .context("Failed to build Tokio runtime")?;
            let approved = verify_config(cfg)?;
            rt.block_on(crate::commands::mcp::run(command, approved))?;
            Ok(ExitCode::SUCCESS)
        }
        None => unreachable!("Clap should handle required subcommands"),
    }
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
    /// Build the ClaudeCode agent's Docker image
    Claudecode {
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
    /// Start an interactive ClaudeCode session
    #[command(alias = "c", disable_help_flag = true)]
    Claudecode {
        /// Extra arguments to pass to the claude command
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
    /// Drop into an interactive shell in the ClaudeCode container
    Claudecode,
}

#[cfg(feature = "mcp")]
#[derive(Subcommand)]
pub enum McpCommands {
    /// Start the MCP HTTP server
    Start {
        /// Port to listen on (overrides cast.json mcp.port)
        #[arg(long)]
        port: Option<u16>,

        /// Host to bind to (overrides cast.json mcp.hostname)
        #[arg(long)]
        host: Option<String>,
    },
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
        #[command(flatten)]
        flags: RunFlags,
        #[command(subcommand)]
        agent: RunAgent,
    },
    /// Drop into an interactive shell in an agent's container
    Shell {
        /// Skip Nix devshell wrapping and open a bare shell
        #[arg(long)]
        raw: bool,
        #[command(subcommand)]
        agent: ShellAgent,
    },
    #[cfg(feature = "mcp")]
    /// Start the MCP server to expose tools to coding agents
    Mcp {
        #[command(subcommand)]
        command: McpCommands,
    },
}

impl RunAgent {
    pub fn as_agent(&self) -> &'static dyn Agent {
        match self {
            RunAgent::Opencode { .. } => &OpenCode,
            RunAgent::Pi { .. } => &Pi,
            RunAgent::Claudecode { .. } => &ClaudeCode,
        }
    }
}

impl ShellAgent {
    pub fn as_agent(&self) -> &'static dyn Agent {
        match self {
            ShellAgent::Opencode => &OpenCode,
            ShellAgent::Pi => &Pi,
            ShellAgent::Claudecode => &ClaudeCode,
        }
    }
}

/// Convert an ExitStatus into an ExitCode, following Unix conventions.
pub fn to_exit_code(status: ExitStatus) -> ExitCode {
    use std::os::unix::process::ExitStatusExt;

    let code = status.code().unwrap_or_else(|| {
        // If terminated by a signal, follow the 128 + signal shell convention
        status.signal().map(|s| 128 + s).unwrap_or(1)
    });

    ExitCode::from(code as u8)
}
