use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Process-isolated, supervised headless launcher for agent harnesses.
#[derive(Debug, Parser)]
#[command(name = "cast-agent", version, about)]
struct Cli {
    #[command(subcommand)]
    command: CommandKind,
}

#[derive(Debug, Subcommand)]
enum CommandKind {
    /// Launch a harness in headless mode and supervise it to completion.
    Run(RunArgs),
}

#[derive(Debug, Parser)]
struct RunArgs {
    /// Which harness to launch. Required; selects the adapter (binary,
    /// JSON dialect, result extractor).
    #[arg(long, value_enum)]
    harness: HarnessKind,

    /// Read the prompt from a file (highest precedence).
    #[arg(long)]
    file: Option<PathBuf>,

    /// Wall-clock timeout in seconds before the child's process group is
    /// killed.
    #[arg(long, default_value_t = 300)]
    timeout: u64,

    /// Override the base directory for the per-run artifact directory.
    #[arg(long)]
    run_dir: Option<PathBuf>,

    /// Inline prompt (lowest precedence; --file > stdin > positional).
    prompt: Option<String>,
}

/// The set of supported harnesses. The MVP ships opencode only; claudecode
/// and pi are deferred (their JSON event shapes / flags remain unverified).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum HarnessKind {
    Opencode,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        CommandKind::Run(_args) => todo!("orchestrator wired in Slice 1c"),
    }
}
