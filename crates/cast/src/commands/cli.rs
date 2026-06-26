use std::process::{ExitCode, ExitStatus};

use super::{config, nix_daemon, port};
use crate::config::{load_config, ApprovedConfig, Config};
use crate::dev;
use crate::dev::agent::Agent;
use crate::dev::claudecode::ClaudeCode;
use crate::dev::opencode::OpenCode;
use crate::dev::pi::Pi;
use crate::dev::run::{RunMode, SessionFlags};
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

    /// Publish the agent's port to the host (uses the calculated port).
    /// To use a specific host port, set `port` in cast.json instead.
    #[arg(short = 'p', long)]
    pub publish: bool,
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
                publish: flags.publish,
            };
            let extra_args = match &agent {
                RunAgent::Opencode { extra_args } => extra_args.clone(),
                RunAgent::Pi { extra_args } => extra_args.clone(),
                RunAgent::Claudecode { extra_args } => extra_args.clone(),
            };
            let status = dev::run_agent(agent.as_agent(), &approved, session_flags, extra_args)?;
            Ok(to_exit_code(status))
        }
        Some(Commands::Exec { flags, agent }) => {
            let approved = verify_config(cfg)?;
            // TTY mode is determined by --headless, independent of naming.
            let mode = if flags.headless {
                RunMode::Headless {
                    token: invocation_id.clone(),
                }
            } else {
                RunMode::Interactive
            };
            // Container name token is always set for exec sessions so that
            // exec containers never collide with interactive `cast run`:
            //   interactive exec → "exec-{id}"
            //   headless exec    → "{id}"
            let name_token = if flags.headless {
                invocation_id.clone()
            } else {
                format!("exec-{}", invocation_id)
            };
            let session_flags = SessionFlags {
                mode,
                name: flags.name.clone(),
                publish: flags.publish,
            };
            let cmd = agent.cmd().to_vec();
            let status = dev::exec(
                agent.as_agent(),
                &approved,
                session_flags,
                flags.raw,
                name_token,
                cmd,
            )?;
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

/// Flags that control the execution mode of `cast exec`.
#[derive(clap::Args, Clone, Debug)]
pub struct ExecFlags {
    /// Run without a TTY (for CI, systemd, and piped output)
    #[arg(long)]
    pub headless: bool,

    /// Override the container name (default: auto-generated)
    #[arg(long)]
    pub name: Option<String>,

    /// Publish the agent's port to the host (uses the calculated port).
    /// To use a specific host port, set `port` in cast.json instead.
    #[arg(short = 'p', long)]
    pub publish: bool,

    /// Skip Nix devshell wrapping; command wrapping is skipped but /nix is
    /// still mounted and the Nix daemon is still started.
    #[arg(long)]
    pub raw: bool,
}

/// Agent subcommands for `cast exec`.
#[derive(Subcommand)]
#[command(subcommand_required = true)]
pub enum ExecAgent {
    /// Execute a command in a fresh OpenCode container
    #[command(alias = "o", disable_help_flag = true)]
    Opencode {
        /// Command and arguments to run inside the container
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 1..)]
        cmd: Vec<String>,
    },
    /// Execute a command in a fresh Pi container
    #[command(alias = "p", disable_help_flag = true)]
    Pi {
        /// Command and arguments to run inside the container
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 1..)]
        cmd: Vec<String>,
    },
    /// Execute a command in a fresh ClaudeCode container
    #[command(alias = "c", disable_help_flag = true)]
    Claudecode {
        /// Command and arguments to run inside the container
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 1..)]
        cmd: Vec<String>,
    },
}

impl ExecAgent {
    pub fn as_agent(&self) -> &'static dyn Agent {
        match self {
            ExecAgent::Opencode { .. } => &OpenCode,
            ExecAgent::Pi { .. } => &Pi,
            ExecAgent::Claudecode { .. } => &ClaudeCode,
        }
    }

    pub fn cmd(&self) -> &[String] {
        match self {
            ExecAgent::Opencode { cmd } => cmd,
            ExecAgent::Pi { cmd } => cmd,
            ExecAgent::Claudecode { cmd } => cmd,
        }
    }
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
    /// Execute an arbitrary command in a fresh agent container
    Exec {
        #[command(flatten)]
        flags: ExecFlags,
        #[command(subcommand)]
        agent: ExecAgent,
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── cast exec clap parsing ───────────────────────────────────────────────

    #[test]
    fn test_exec_opencode_parses_cmd() {
        let cli =
            Cli::try_parse_from(["cast", "exec", "opencode", "/bin/bash"]).expect("should parse");
        let Commands::Exec { agent, flags } = cli.command.unwrap() else {
            panic!("expected Exec command");
        };
        let ExecAgent::Opencode { cmd } = agent else {
            panic!("expected Opencode agent");
        };
        assert_eq!(cmd, vec!["/bin/bash"]);
        assert!(!flags.headless);
        assert!(!flags.raw); // off by default
    }

    #[test]
    fn test_exec_requires_cmd_arg() {
        // `cast exec opencode` with no cmd: clap's trailing_var_arg does not
        // enforce num_args minimum at parse time; the empty Vec is caught at
        // dispatch time. Verify the parse succeeds but cmd is empty.
        let cli = Cli::try_parse_from(["cast", "exec", "opencode"]).expect("parses ok");
        let Commands::Exec { agent, .. } = cli.command.unwrap() else {
            panic!("expected Exec");
        };
        assert!(agent.cmd().is_empty(), "cmd should be empty with no args");
    }

    #[test]
    fn test_exec_raw_flag_parsed() {
        let cli = Cli::try_parse_from([
            "cast",
            "exec",
            "--raw",
            "opencode",
            "/bin/bash",
            "-c",
            "echo hi",
        ])
        .expect("should parse");
        let Commands::Exec { agent, flags } = cli.command.unwrap() else {
            panic!("expected Exec command");
        };
        let ExecAgent::Opencode { cmd } = agent else {
            panic!("expected Opencode");
        };
        assert!(flags.raw);
        assert_eq!(cmd, vec!["/bin/bash", "-c", "echo hi"]);
    }

    #[test]
    fn test_exec_headless_flag_parsed() {
        let cli = Cli::try_parse_from(["cast", "exec", "--headless", "opencode", "/bin/bash"])
            .expect("should parse");
        let Commands::Exec { flags, .. } = cli.command.unwrap() else {
            panic!("expected Exec command");
        };
        assert!(flags.headless);
    }

    #[test]
    fn test_exec_publish_flag_sets_true() {
        // Bare --publish flag (no value) sets publish to true.
        let cli = Cli::try_parse_from(["cast", "exec", "--publish", "opencode", "/bin/bash"])
            .expect("should parse");
        let Commands::Exec { flags, .. } = cli.command.unwrap() else {
            panic!("expected Exec command");
        };
        assert!(
            flags.publish,
            "--publish bare flag must set publish to true"
        );
    }

    #[test]
    fn test_run_publish_flag_sets_true() {
        let cli =
            Cli::try_parse_from(["cast", "run", "--publish", "opencode"]).expect("should parse");
        let Commands::Run { flags, .. } = cli.command.unwrap() else {
            panic!("expected Run command");
        };
        assert!(
            flags.publish,
            "--publish bare flag must set publish to true"
        );
    }

    #[test]
    fn test_publish_absent_is_false() {
        let cli =
            Cli::try_parse_from(["cast", "exec", "opencode", "/bin/bash"]).expect("should parse");
        let Commands::Exec { flags, .. } = cli.command.unwrap() else {
            panic!("expected Exec command");
        };
        assert!(!flags.publish, "publish absent should be false");
    }

    #[test]
    fn test_exec_pi_alias_parses() {
        let cli =
            Cli::try_parse_from(["cast", "exec", "p", "/bin/bash"]).expect("alias p should parse");
        let Commands::Exec { agent, .. } = cli.command.unwrap() else {
            panic!("expected Exec command");
        };
        assert!(matches!(agent, ExecAgent::Pi { .. }));
    }

    #[test]
    fn test_exec_flags_precede_agent() {
        // Flags MUST come before the agent subcommand; after the agent they
        // are part of cmd and are not parsed as flags.
        let cli = Cli::try_parse_from(["cast", "exec", "opencode", "--raw", "/bin/bash"])
            .expect("should parse");
        let Commands::Exec { agent, flags } = cli.command.unwrap() else {
            panic!("expected Exec command");
        };
        // --raw appearing after the agent subcommand is captured as part of cmd
        let ExecAgent::Opencode { cmd } = agent else {
            panic!("expected Opencode");
        };
        assert!(
            !flags.raw,
            "raw flag after agent should NOT be parsed as flag"
        );
        assert_eq!(cmd[0], "--raw", "raw after agent should be in cmd");
    }
}
