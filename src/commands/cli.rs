use std::process::{ExitCode, ExitStatus};

use super::{config, nix_daemon, port};
use crate::config::{load_config, ApprovedConfig, Config};
use crate::dev;
use crate::dev::agent::Agent;
use crate::dev::opencode::OpenCode;
use crate::dev::pi::Pi;
use crate::dev::workspace::get_workspace;
use crate::logging::{generate_invocation_id, init_file_logger};
use crate::user::get_user;
use anyhow::{Context as _, Result};
use clap::{Parser, Subcommand};
use tracing::info_span;

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
        Some(Commands::Config { command }) => config::handle_config(&cfg, command),
        Some(Commands::NixDaemon { command }) => {
            let approved = verify_config(cfg)?;
            nix_daemon::handle_nix_daemon(&approved, command)
        }
        Some(Commands::Port { agent }) => port::handle_port(&cfg, agent.as_agent()),
        Some(Commands::Run { agent }) => {
            let approved = verify_config(cfg)?;
            let status = dev::run_agent(
                agent.as_agent(),
                &approved,
                match &agent {
                    RunAgent::Opencode { extra_args } => extra_args.clone(),
                    RunAgent::Pi { extra_args } => extra_args.clone(),
                },
            )?;
            Ok(to_exit_code(status))
        }
        Some(Commands::Shell {
            agent: ShellAgent::Opencode,
        }) => {
            let approved = verify_config(cfg)?;
            let status = dev::shell(&OpenCode, &approved)?;
            Ok(to_exit_code(status))
        }
        Some(Commands::Shell {
            agent: ShellAgent::Pi,
        }) => {
            let approved = verify_config(cfg)?;
            let status = dev::shell(&Pi, &approved)?;
            Ok(to_exit_code(status))
        }
        #[cfg(feature = "mcp")]
        Some(Commands::Mcp { command }) => {
            use crate::commands::cli::McpCommands;
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .context("Failed to build Tokio runtime")?;
            match command {
                McpCommands::List { url } => {
                    rt.block_on(crate::commands::mcp::list_tools(url))?;
                }
                McpCommands::Describe { tool_name, url } => {
                    rt.block_on(crate::commands::mcp::describe_tool(tool_name, url))?;
                }
                McpCommands::Call {
                    tool_name,
                    params,
                    url,
                } => {
                    rt.block_on(crate::commands::mcp::call_tool_cmd(tool_name, params, url))?;
                }
                other => {
                    let approved = verify_config(cfg)?;
                    rt.block_on(crate::commands::mcp::run(other, approved))?;
                }
            }
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
    /// List tools exposed by the MCP server
    List {
        /// MCP server URL (overrides CAST_MCP_URL env and default)
        #[arg(long)]
        url: Option<String>,
    },
    /// Show the input schema for a specific MCP tool
    Describe {
        /// Name of the tool to inspect
        tool_name: String,

        /// MCP server URL (overrides CAST_MCP_URL env and default)
        #[arg(long)]
        url: Option<String>,
    },
    /// Call a tool on the MCP server with JSON arguments
    Call {
        /// Name of the tool to call
        tool_name: String,

        /// JSON arguments as an inline string, or '-' to read from stdin.
        /// Defaults to '{}' if omitted and stdin is a terminal.
        #[arg(value_name = "JSON")]
        params: Option<String>,

        /// MCP server URL (overrides CAST_MCP_URL env and default)
        #[arg(long)]
        url: Option<String>,
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
        #[command(subcommand)]
        agent: RunAgent,
    },
    /// Drop into an interactive shell in an agent's container
    Shell {
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
