# Project Log

## [4797102] Documented architectural decision: Reject bollard in favor of Docker CLI

- **Decided:** Keep docker CLI and reject bollard for cast, based on TTY handling and environment compatibility.

## [4797102] Planned child process refactor for interactive Docker commands

- **Found:** pre_exec reset is strictly required: POSIX preserves SIG_IGN across exec(), so without it docker inherits the ignore disposition
- **Found:** SIGQUIT (Ctrl+backslash) needs same treatment as SIGINT -- both are TTY foreground process group signals
- **Found:** Three call sites all need the same fix: run.rs, shell.rs, nix_daemon/daemon.rs
- **Decided:** Replace execvp (exec_command) with supervised child process (interactive_command) using .status()
- **Decided:** Ignore SIGINT and SIGQUIT in cast parent before spawn; reset to SIG_DFL in child via pre_exec
- **Decided:** SIGTSTP and SIGTERM left at SIG_DFL -- no supervisor forwarding needed for basic CLI use
- **Decided:** Add libc dependency for signal manipulation -- already transitive, zero overhead
- **Decided:** Exit code propagated via std::process::exit(status.code().unwrap_or(1))
- **Decided:** ctrlc crate rejected -- SIG_IGN is the correct primitive; ctrlc adds unnecessary overhead for this use case

## [4797102-dirty] Implemented supervised child process for Docker interactive sessions

Refactored DockerClient to use .status() instead of execvp for interactive commands. Added signal handling to ignore SIGINT/SIGQUIT in parent and reset in child. Propagated exit codes throughout the CLI layer.

- **Found:** TTY passthrough works as expected with Command::status()
- **Found:** cargo check and cargo test confirm type safety and no regressions in existing logic
- **Decided:** Using Result<ExitStatus> as the return type for run_agent and shell functions to allow flexible exit code propagation

## [84a96ad] Code Review: Supervised child process refactor

Consultant Gemini review confirmed the architecture but identified two robustness issues with signal handling and exit code propagation.

- **Found:** Signal restoration is skipped if cmd.status() returns an error
- **Found:** status.code().unwrap_or(1) loses Unix signal termination information (128 + signal convention)
- **Decided:** Implement a SignalGuard (RAII) to ensure signal handlers are restored even on failure
- **Decided:** Use ExitStatusExt to correctly propagate signal termination exit codes

## [8c37f0f-dirty] Completed Phase 1 (Child Process Refactor) and planned Phase 2 (Instrumentation)

The supervised child process model is now robustly implemented with RAII signal handling and accurate exit code propagation. Updated all spec files to define the integration plan for the tracing ecosystem.

- **Decided:** Use a two-layer tracing strategy: stderr for INFO progress, file-based JSON for DEBUG diagnostics.
- **Decided:** Target platform-specific state directories for log storage (e.g., ~/.local/state/cast/).
- **Open:** Final choice of log rotation policy and retention limits.

## [8c37f0f-dirty] Designed instrumentation architecture for Phase 2

- **Found:** Daily rotation at 15 projects x 3 agents x 30 days = 1350+ files/month -- rejected
- **Found:** Single shared file breaks multi-line log entries (panics, backtraces) -- confirmed by Gemini
- **Found:** tracing_appender::rolling::never + manual size truncation is the correct pattern for this use case
- **Decided:** File-only logging: no console tracing layer; existing println! messages stay as intentional UX
- **Decided:** Per-project/per-agent stable log files at ~/.local/share/cast/logs/<project>/<agent>.log
- **Decided:** Size-based truncation (5 MB -> rename to .log.old) instead of time-based rotation
- **Decided:** rolling::never + synchronous writes; no non_blocking to avoid WorkerGuard dropped-log risk
- **Decided:** Plain text format with with_ansi(false); INFO default; DEBUG via CAST_LOG=debug
- **Decided:** Single info_span! wrapping run_agent body to capture session duration and context fields

## [41ada38] Implemented Phase 2 and 3 of Instrumentation

- **Found:** Successfully integrated tracing with file-based rotation and instrumented core modules.
- **Decided:** Used tracing-subscriber and tracing-appender for logging; added spans and events to run_agent and DockerClient.

## [d927f1e] Revised log architecture: single daily file + RandomState invocation ID

- **Found:** O_APPEND on regular files is serialized at kernel inode level -- concurrent processes safe (Gemini confirmed)
- **Found:** rolling::daily uses timestamp-suffixed filenames, not rename-based rotation -- no midnight race (Gemini confirmed)
- **Found:** PID recycling is a real risk on dev machines (pid_max=32768, exhausted within a day by build tools)
- **Found:** RandomState is seeded by OS entropy -- safer than nanosecond timestamp for parallel launch collision
- **Decided:** Single shared daily-rotated file for all cast activity (Option B)
- **Decided:** Per-project/per-agent files rejected: 1000+ files/month at typical usage scale
- **Decided:** RandomState (std::collections::hash_map) used for invocation ID: OS-seeded, zero new deps, collision-proof
- **Decided:** Root span carries both id (8 hex chars, for grep) and pid (for ps/htop cross-reference)
- **Decided:** tracing_appender::rolling::daily passed directly to with_writer() -- no non_blocking, no WorkerGuard

## [0f2cc67] Implemented structured logging and instrumentation

- **Found:** Synchronous file logging is preferred for CLI tools to ensure flush on exit.
- **Decided:** Used tracing-appender for daily rotation and deterministic invocation IDs for log correlation.
- **Open:** Add more granular spans for network operations if performance issues arise.

