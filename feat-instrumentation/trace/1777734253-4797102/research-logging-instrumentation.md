# Research: Lack of Logging and Instrumentation in `cast`

## Current Situation
The `cast` application currently lacks any formal logging, instrumentation, or crash reporting. Informal progress messages are printed directly to `stdout`/`stderr` using `println!`.

## Identified Problems
1. **Process Hijacking (execvp)**: The `run` and `shell` commands use `execvp` to replace the `cast` process with `docker`. This prevents `cast` from monitoring the agent's lifecycle, capturing exit codes, or performing post-execution analysis.
2. **No Persistent Logs**: Errors and status updates are transient. If a failure occurs in a non-interactive environment or a container crashes silently, there is no log file to inspect.
3. **Lack of Visibility**: Critical operations (config loading, image building, container orchestration) are opaque at the `DEBUG` level.
4. **Resource Constraints**: Failures due to `memory`, `cpus`, or `pids_limit` settings are hard to correlate without seeing the exact arguments passed to Docker and the resulting system state.

## Research Findings
- `src/docker/client.rs` uses `std::process::Command` but lacks structured logging of the commands being executed.
- `src/commands/cli.rs` and `src/dev/run.rs` orchestrate the main execution flow without spans or event tracking.
- `main.rs` only reports errors to `stderr` at the very end of the process (if `execvp` wasn't called).

## Proposed Solution (Summary)
- Integrate `tracing` and `tracing-appender` for persistent file logging.
- Refactor `exec_command` to maintain `cast`'s execution throughout the agent's lifetime.
- Implement structured spans for key lifecycle events.
