use std::process::{ExitCode, ExitStatus};

use super::{config, nix_daemon, port};
use crate::config::{ApprovedConfig, Config, load_config, load_config_from};
use crate::dev;
use crate::dev::agent::Agent;
use crate::dev::claudecode::ClaudeCode;
use crate::dev::opencode::OpenCode;
use crate::dev::pi::Pi;
use crate::dev::service_context::ServiceContext;
use crate::dev::workspace::get_workspace;
use crate::logging::{generate_invocation_id, init_file_logger};
use crate::user::get_user;
use anyhow::{Context as _, Result};
use clap::{Parser, Subcommand};
use tracing::info_span;

/// Selects a service within the current Git worktree.
#[derive(clap::Args, Clone, Debug)]
pub struct ServiceFlags {
    /// Select a named service instead of the default service
    #[arg(long)]
    pub name: Option<String>,
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
    // Initialization must work even when an existing global config is invalid:
    // the command never overwrites it and can still create a missing flake.
    if matches!(
        &cli.command,
        Some(Commands::Config {
            command: Some(config::ConfigCommands::Init)
        })
    ) {
        config::init_global_config()?;
        return Ok(ExitCode::SUCCESS);
    }

    // Service commands are scoped to the Git worktree, even when invoked from
    // one of its subdirectories. Other commands retain their existing cwd
    // configuration scope.
    let cfg = if matches!(
        &cli.command,
        Some(
            Commands::Up { .. }
                | Commands::Down { .. }
                | Commands::Status { .. }
                | Commands::Exec { .. }
                | Commands::Shell { .. }
        )
    ) {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        let context = ServiceContext::resolve(&cwd)?;
        load_config_from(&context.worktree_root)?
    } else {
        load_config()?
    };

    // Initialize file logger
    init_file_logger()?;

    let invocation_id = generate_invocation_id();
    let root = info_span!("cast", id = %invocation_id, pid = std::process::id());
    let _root_guard = root.enter();

    match cli.command {
        Some(Commands::Build {
            nix_daemon,
            force,
            no_cache,
        }) => {
            let approved = verify_config(cfg)?;
            dev::build_dev_image(&approved, nix_daemon, force, no_cache)?;
            Ok(ExitCode::SUCCESS)
        }
        Some(Commands::Config { command }) => config::handle_config(&cfg, command),
        Some(Commands::NixDaemon { command }) => {
            let approved = verify_config(cfg)?;
            nix_daemon::handle_nix_daemon(&approved, command)
        }
        Some(Commands::Port { agent }) => port::handle_port(&cfg, agent.as_agent()),
        Some(Commands::Up { flags }) => {
            let cwd = std::env::current_dir().context("Failed to get current directory")?;
            let context = ServiceContext::resolve(&cwd)?;
            let approved = crate::config::check_approved(cfg, &context.worktree_root)?;
            dev::service::up(&approved, &context, flags.name.as_deref())?;
            Ok(ExitCode::SUCCESS)
        }
        Some(Commands::Down { flags }) => {
            let cwd = std::env::current_dir().context("Failed to get current directory")?;
            let context = ServiceContext::resolve(&cwd)?;
            dev::service::down(&context, flags.name.as_deref())?;
            Ok(ExitCode::SUCCESS)
        }
        Some(Commands::Status { flags }) => {
            let cwd = std::env::current_dir().context("Failed to get current directory")?;
            let context = ServiceContext::resolve(&cwd)?;
            let service_status = dev::service::status(&context, flags.name.as_deref())?;
            println!(
                "{}: {}",
                context.container_name(flags.name.as_deref()),
                service_status
            );
            Ok(ExitCode::SUCCESS)
        }
        Some(Commands::Exec { flags: _, cmd: _ }) => {
            anyhow::bail!("cast exec is not implemented yet")
        }
        Some(Commands::Shell { flags: _, raw: _ }) => {
            anyhow::bail!("cast shell is not implemented yet")
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

/// Flags that control the execution mode of `cast exec`.
#[derive(clap::Args, Clone, Debug)]
pub struct ExecFlags {
    /// Run without a TTY (for CI, systemd, and piped output)
    #[arg(long)]
    pub headless: bool,

    /// Select a named service instead of the default service
    #[arg(long)]
    pub name: Option<String>,

    /// Skip Nix devshell wrapping; command wrapping is skipped but /nix is
    /// still mounted and the Nix daemon is still started.
    #[arg(long)]
    pub raw: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Build the shared dev image
    Build {
        /// Also build the Nix daemon image
        #[arg(long)]
        nix_daemon: bool,
        /// Force rebuild even if image already exists
        #[arg(short, long)]
        force: bool,
        /// Do not use Docker cache
        #[arg(long)]
        no_cache: bool,
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
    /// Start the service container
    Up {
        #[command(flatten)]
        flags: ServiceFlags,
    },
    /// Stop and remove the service container
    Down {
        #[command(flatten)]
        flags: ServiceFlags,
    },
    /// Show the service container status
    Status {
        #[command(flatten)]
        flags: ServiceFlags,
    },
    /// Execute an arbitrary command in the service container
    Exec {
        #[command(flatten)]
        flags: ExecFlags,
        /// Command and arguments to execute
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        cmd: Vec<String>,
    },
    /// Drop into an interactive shell in the service container
    Shell {
        #[command(flatten)]
        flags: ServiceFlags,
        /// Skip Nix devshell wrapping and open a bare shell
        #[arg(long)]
        raw: bool,
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

/// Convert an ExitStatus into an ExitCode, following Unix conventions.
pub fn to_exit_code(status: ExitStatus) -> ExitCode {
    use std::os::unix::process::ExitStatusExt;

    let code = status.code().unwrap_or_else(|| {
        // If terminated by a signal, follow the 128 + signal shell convention
        status.signal().map(|s| 128 + s).unwrap_or(1)
    });

    ExitCode::from(code as u8)
}
