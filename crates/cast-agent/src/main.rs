use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::time::Duration;

use cast_agent::harness::{Harness, OpenCode};
use cast_agent::prompt::resolve_prompt;
use cast_agent::run::{limits_from_timeout, orchestrate};
use cast_agent::rundir::{create_run_dir, resolve_base_from_env};

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

    /// Select the harness-side persona/agent config. For opencode this maps to
    /// `opencode run --agent <name>` (persona prompt + permission profile +
    /// model). An unsupported harness rejects this flag rather than ignoring it.
    #[arg(long)]
    agent: Option<String>,

    /// Inline prompt (lowest precedence; --file > stdin > positional).
    prompt: Option<String>,
}

/// The set of supported harnesses. The MVP ships opencode only; claudecode
/// and pi are deferred (their JSON event shapes / flags remain unverified).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum HarnessKind {
    Opencode,
}

impl HarnessKind {
    fn adapter(self) -> Box<dyn Harness> {
        match self {
            HarnessKind::Opencode => Box::new(OpenCode),
        }
    }
}

/// Exit code for a usage error (bad args / unreadable prompt) — emitted before
/// any run begins, so it lives here rather than in the outcome table.
const EXIT_USAGE: i32 = 2;

#[cfg(unix)]
#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let code = match cli.command {
        CommandKind::Run(args) => run(args).await,
    };
    std::process::exit(code);
}

#[cfg(unix)]
async fn run(args: RunArgs) -> i32 {
    let harness = args.harness.adapter();

    // Resolve the prompt first: a usage error here must not create a run dir.
    let prompt = match resolve_prompt(args.file.as_deref(), args.prompt) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("cast-agent: {e}");
            return EXIT_USAGE;
        }
    };

    let base = resolve_base_from_env(args.run_dir);
    let run_dir = match create_run_dir(&base, harness.name()) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("cast-agent: could not create run directory: {e}");
            return EXIT_USAGE;
        }
    };
    // The run-dir is the addressable control+recovery handle — announce it
    // (and our PID) on stderr before any child output so an operator can tail.
    eprintln!("cast-agent: run-dir {}", run_dir.display());
    eprintln!("cast-agent: pid {}", std::process::id());

    let limits = limits_from_timeout(Duration::from_secs(args.timeout));
    // Resolve the harness the declared way. Binary-level integration tests
    // exercise the spawn path by putting a shim named `opencode` first on
    // PATH (the helper in `tests/interrupt_test.rs`); there is no in-process
    // override, so `result.json.harness` always names what actually ran.
    let exe = harness.base_command().to_string();
    let mut cmd_args = harness.headless_args();
    // Append persona selection. If the operator asked for an --agent but the
    // harness does not support it, reject loudly rather than silently dropping
    // it (a silent drop is a quiet permission/scope regression: the run would
    // launch the default agent with full tool access).
    if let Some(name) = args.agent.as_deref() {
        match harness.agent_args(name) {
            Some(extra) => cmd_args.extend(extra),
            None => {
                eprintln!(
                    "cast-agent: harness {} does not support --agent",
                    harness.name()
                );
                return EXIT_USAGE;
            }
        }
    }

    match orchestrate(harness.as_ref(), &exe, &cmd_args, &prompt, &run_dir, limits).await {
        Ok(report) => {
            if let Some(msg) = report.final_message {
                println!("{msg}");
            }
            report.exit_code
        }
        Err(e) => {
            // orchestrate now maps runtime/supervision failures to a Crashed
            // verdict internally; a returned Err is a pre-run setup failure
            // (e.g. persisting prompt.txt / cast-agent.pid failed), which is a
            // usage-class error with no run to report.
            eprintln!("cast-agent: setup failed: {e}");
            EXIT_USAGE
        }
    }
}

#[cfg(not(unix))]
fn main() {
    eprintln!("cast-agent is only supported on unix platforms");
    std::process::exit(2);
}
