use std::process::{ExitCode, ExitStatus};

use super::{config, nix_daemon};
use crate::config::{ApprovedConfig, Config, load_config, load_config_from};
use crate::dev;
use crate::dev::instance_context::InstanceContext;
use crate::dev::workspace::get_workspace;
use crate::logging::{generate_invocation_id, init_file_logger};
use crate::user::get_user;
use anyhow::{Context as _, Result};
use clap::{Parser, Subcommand};
use tracing::info_span;

/// Accepts names that keep the derived container name valid for Docker:
/// `[a-zA-Z0-9][a-zA-Z0-9_.-]*`.
fn parse_instance_name(name: &str) -> Result<String, String> {
    let mut chars = name.chars();
    let valid = chars.next().is_some_and(|c| c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c));
    if valid {
        Ok(name.to_string())
    } else {
        Err("use letters, digits, '_', '.' or '-', starting with a letter or digit".to_string())
    }
}

/// Selects an instance within the current Git worktree.
#[derive(clap::Args, Clone, Debug)]
pub struct InstanceFlags {
    /// Select a named instance instead of the default instance
    #[arg(long, value_parser = parse_instance_name)]
    pub name: Option<String>,
}

/// cast - coding agent sandbox tool
#[derive(Parser)]
#[command(name = "cast")]
#[command(about, long_about = None, version)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub struct Cli {
    /// Run as if cast was started in <DIR> instead of the current directory
    #[arg(short = 'C', global = true, value_name = "DIR")]
    pub directory: Option<std::path::PathBuf>,

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
    // Like `git -C`: every later cwd lookup sees <DIR>.
    if let Some(dir) = &cli.directory {
        std::env::set_current_dir(dir)
            .with_context(|| format!("Cannot change to directory {}", dir.display()))?;
    }

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

    // Instance commands are scoped to the Git worktree, even when invoked from
    // one of its subdirectories. Other commands retain their existing cwd
    // configuration scope.
    let cfg = if matches!(&cli.command, Some(Commands::Down { .. } | Commands::Ps)) {
        Config::default()
    } else if matches!(
        &cli.command,
        Some(Commands::Up { .. } | Commands::Exec { .. } | Commands::Shell { .. })
    ) {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        let context = InstanceContext::resolve(&cwd)?;
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
        Some(Commands::Up { flags }) => {
            let cwd = std::env::current_dir().context("Failed to get current directory")?;
            let context = InstanceContext::resolve(&cwd)?;
            let approved = crate::config::check_approved(cfg, &context.worktree_root)?;
            dev::instance::up(&approved, &context, flags.name.as_deref())?;
            Ok(ExitCode::SUCCESS)
        }
        Some(Commands::Down { flags }) => {
            let cwd = std::env::current_dir().context("Failed to get current directory")?;
            let context = InstanceContext::resolve(&cwd)?;
            dev::instance::down(&context, flags.name.as_deref())?;
            Ok(ExitCode::SUCCESS)
        }
        Some(Commands::Ps) => {
            let listing = dev::ps::list(&crate::docker::client::DockerClient)?;
            println!("{}", serde_json::to_string_pretty(&listing)?);
            Ok(ExitCode::SUCCESS)
        }
        Some(Commands::Exec { flags, cmd }) => {
            let cwd = std::env::current_dir().context("Failed to get current directory")?;
            let context = InstanceContext::resolve(&cwd)?;
            let approved = crate::config::check_approved(cfg, &context.worktree_root)?;
            let status = dev::exec(
                &approved,
                &context,
                flags.name.as_deref(),
                flags.headless,
                flags.raw,
                cmd,
            )?;
            Ok(to_exit_code(status))
        }
        Some(Commands::Shell { flags, raw }) => {
            let cwd = std::env::current_dir().context("Failed to get current directory")?;
            let context = InstanceContext::resolve(&cwd)?;
            let approved = crate::config::check_approved(cfg, &context.worktree_root)?;
            let status = dev::shell(&approved, &context, flags.name.as_deref(), raw)?;
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

    /// Select a named instance instead of the default instance
    #[arg(long, value_parser = parse_instance_name)]
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
    /// Start an instance container
    Up {
        #[command(flatten)]
        flags: InstanceFlags,
    },
    /// Stop and remove an instance container
    Down {
        #[command(flatten)]
        flags: InstanceFlags,
    },
    /// List running instances across all workspaces as JSON
    Ps,
    /// Execute an arbitrary command in an instance container
    Exec {
        #[command(flatten)]
        flags: ExecFlags,
        /// Command and arguments to execute
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        cmd: Vec<String>,
    },
    /// Drop into an interactive shell in an instance container
    Shell {
        #[command(flatten)]
        flags: InstanceFlags,
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

/// Convert an ExitStatus into an ExitCode, following Unix conventions.
pub fn to_exit_code(status: ExitStatus) -> ExitCode {
    use std::os::unix::process::ExitStatusExt;

    let code = status.code().unwrap_or_else(|| {
        // If terminated by a signal, follow the 128 + signal shell convention
        status.signal().map(|s| 128 + s).unwrap_or(1)
    });

    ExitCode::from(code as u8)
}
