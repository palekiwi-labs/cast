# feat-instrumentation: Logging and Instrumentation

## Background

The `cast` application currently lacks formal logging and instrumentation. Progress messages are
printed directly to `stdout`/`stderr` using `println!`, and because the application previously used
`execvp` to launch agent sessions, it was impossible to monitor the container's lifecycle or capture
accurate exit statuses.

## Goal

Implement a robust logging and instrumentation system using the `tracing` ecosystem. This will provide
better visibility into the application's internals, improve debuggability, and ensure that all
critical operations (image building, agent sessions, Nix daemon management) are tracked.

## Scope

- **Phase 1 (Completed)**: Refactor interactive Docker sessions to use a supervised child process
  instead of `execvp`.
- **Phase 2**: Integrate `tracing` and `tracing-appender` for persistent file logging and
  structured console output.
- **Phase 3**: Instrument key modules (`src/docker/`, `src/dev/`, `src/config/`) with spans and
  events.

## Constraints

- Support standard Unix signal conventions for exit codes (128 + signal). (Completed)
- Ensure TTY interactive behavior is preserved. (Completed)
- Log files stored at `~/.local/share/cast/logs/` (XDG data dir via `dirs` crate).
- No console tracing layer — existing `println!` messages are intentional UX and remain.
- File logging only; plain text format; synchronous writes; no `non_blocking` wrapper.
- Per-project, per-agent stable log files with size-based truncation (not time-based rotation).
