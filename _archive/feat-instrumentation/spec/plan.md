# Plan: Logging and Instrumentation Integration

## Phase 1: Supervised Child Process (Completed)

`execvp` was replaced with a supervised child process (`Command::status`). Key implementation
details recorded in the log:

- `SignalGuard` RAII struct ignores `SIGINT`/`SIGQUIT` in the parent and restores them on drop.
- `pre_exec` hook resets both signals to `SIG_DFL` in the child (required: `SIG_IGN` is preserved
  across `exec()` per POSIX).
- `exit_with_status` helper in `src/commands/cli.rs` implements the `128 + signal` Unix convention.

## Phase 2: File Logging with tracing

### Key Design Decisions

**1. File-only logging — no console layer.**
The existing `println!` messages are intentional user-facing UX (progress during slow operations
like image builds). They stay as-is. `tracing` adds structured detail to a log file only.
A console layer would change the output format without benefit and is unnecessary.

**2. Single daily-rotated log file.**
All `cast` invocations across all projects and agents write to one shared file per day.

Rationale (confirmed by Gemini consultation):
- `O_APPEND` writes on regular files are serialized at the kernel inode level — concurrent
  processes produce cleanly interleaved lines with no torn writes or corruption.
- `tracing_appender::rolling::daily` uses timestamp-suffixed filenames (e.g. `cast.2026-05-03`),
  not rename-based rotation. At midnight, all processes independently open the new dated file.
  No race condition, no lost entries.
- Multi-line log entries (panics, backtraces) are safe: `tracing-subscriber` buffers each event
  into memory before issuing a single `write(2)` syscall, so each entry is atomic.

Per-project/per-agent files were considered and rejected: at 10-15 projects × 3 agents × 30 days
= 1,000+ files/month, file proliferation is impractical. A single file with per-invocation
correlation (see below) achieves equivalent debuggability.

**3. Per-invocation correlation via root span.**
Because all invocations share one file, every log line carries a unique invocation `id` and the
process `pid`. This allows exact filtering of a single run (see "Invocation Correlation" below).

**4. Plain text format.**
Sufficient for `cat`/`tail` inspection. Structured fields appear inline on each line.
`with_ansi(false)` is required to prevent ANSI escape codes from polluting the file.

**5. Synchronous writing — no `non_blocking` wrapper.**
`cast` is a short-lived, low-volume CLI tool. Synchronous writes have no meaningful overhead
and eliminate the `WorkerGuard` dropped-log risk entirely. `RollingFileAppender` implements
`MakeWriter` and can be passed directly to `.with_writer()`.

**6. INFO by default; DEBUG via `CAST_LOG=debug`.**
INFO provides a clean session summary (a few lines per invocation). DEBUG exposes full docker
args, mount resolution, and version details for diagnosis.

### Log File Layout

```
~/.local/share/cast/logs/
  cast.2026-05-03        <- all invocations on that day, all projects, all agents
  cast.2026-05-04
  ...
```

`tracing_appender::rolling::daily(log_dir, "cast")` produces this naming automatically.

### Invocation Correlation

Each `cast` process generates a unique 8-hex-char `id` at startup using `RandomState`, which is
seeded by OS entropy (same source as `HashMap` HashDoS protection). This is combined with the raw
process `pid` to give both collision-free grepping and system-tool cross-referencing.

```rust
fn generate_invocation_id() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let random_u64 = RandomState::new().build_hasher().finish();
    format!("{:08x}", random_u64 as u32)
}
```

A root span wrapping the entire `run()` body carries both fields:

```rust
let root = tracing::info_span!("cast", id = %invocation_id, pid = std::process::id());
let _root_guard = root.enter();
```

Every log line from that invocation then reads:
```
2026-05-04T10:23:45Z  INFO cast{id=a3f8b2c1 pid=83241}: session started agent=opencode
```

Correlating one invocation: `grep 'id=a3f8b2c1' ~/.local/share/cast/logs/cast.2026-05-04`

Rationale for `RandomState` over raw PID: PID recycling is a real risk on developer machines
(build tools exhaust `pid_max` = 32,768 within a working day). `RandomState` is OS-seeded,
requires no new dependencies, and cannot collide with parallel launches (unlike nanosecond
timestamps). The `pid` field is retained alongside for `ps`/`htop` cross-referencing.

### Subscriber Initialization Sequence

In `run()` in `src/commands/cli.rs`, before dispatching to any command handler:

1. Load config (already first).
2. Generate invocation ID via `generate_invocation_id()`.
3. Build `tracing_appender::rolling::daily(log_dir, "cast")` appender.
   - `log_dir` = `dirs::data_dir().unwrap_or_else(|| PathBuf::from("~/.local/share")) / "cast/logs"`
4. Parse `CAST_LOG` env var for level filter (default `INFO`).
5. Initialize subscriber:
   `tracing_subscriber::fmt().with_writer(appender).with_ansi(false).with_max_level(level).init()`
6. Enter root span: `info_span!("cast", id = %invocation_id, pid = std::process::id())`.

The root span guard must be held for the remainder of `run()`.

### What to Instrument

#### INFO (always written to file)

- Config loaded: key values (`memory`, `cpus`, `pids_limit`, `network`)
- Nix daemon: "already running" or "starting container"
- Image: "exists, skipping build" or "not found, building"
- Session start: `agent`, `container_name`, `image_tag`, `port`
- Session end: `exit_code`, `duration_secs`
- Errors: log before propagating so the file always captures failures

#### DEBUG (`CAST_LOG=debug` only)

- Full docker args for every `run_command`, `stream_command`, `interactive_command` call
- Port resolution: cksum inputs and result
- Container name derivation: inputs and result
- Shadow mounts: paths resolved, count
- Volume args built
- Version resolution: resolved tag, source (cache/network)

### Session Span Strategy

Use a single `info_span!` wrapping the body of `run_agent` to automatically capture session
duration and attach `agent`/`container`/`port` fields to every event within the session.
This span is nested inside the root invocation span, so all events carry both sets of fields.

```rust
let span = info_span!(
    "agent_session",
    agent = %agent.name(),
    container = %container_name,
    port = %port,
);
let _enter = span.enter();
```

For operations outside of a session (nix daemon, image build) use standalone events, not spans.

### Future: `cast logs` Command

With a single daily file, `cast logs opencode` would:
1. Resolve the container name for the current project + agent (deterministic cksum).
2. Search `~/.local/share/cast/logs/` for recent daily files.
3. Filter lines matching the container name field.
4. Display (optionally paginated, newest first).
