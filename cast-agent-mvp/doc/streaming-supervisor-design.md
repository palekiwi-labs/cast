---
refs:
- .cue/cast-agent-mvp/todo/1785127643-8ca9cdd/streaming-supervision-investigation.md
- .cue/cast-agent-mvp/plan/1785127643-8ca9cdd/phase-1-mvp.md
- .cue/cast-agent-mvp/doc/cast-agent-architecture.md
- .cue/cast-agent-mvp/doc/opencode-headless-run-contract.md
---
# cast-agent — Streaming Supervisor Design (investigation findings)

Resolves the investigation in
`todo/1785127643-8ca9cdd/streaming-supervision-investigation.md`. Consolidates
a source-verification pass over the `cast` repo (explore agent) and a Tokio
concurrency design consultation (gemini-pro). Feeds updated Slice 1c steps in
`plan/1785127643-8ca9cdd/phase-1-mvp.md`.

## Decision summary

Use the **dedicated-reader-tasks (actor) pattern**, NOT a mega
`tokio::select!` loop. The supervisor:

1. Spawns the child in its own process group (`cmd.process_group(0)`), stdin/
   stdout/stderr all piped.
2. Writes the prompt to stdin and drops it (EOF).
3. Takes the stdout/stderr handles and spawns ONE reader task per pipe.
   - stdout task: `BufReader::lines()` -> for each line, append raw + `\n` to
     the run-log file and **`flush().await` per line** (live `tail -f`), then
     `serde_json::from_str` and collect into a `Vec<Value>`.
   - stderr task: drain continuously (prevents pipe-buffer deadlock); tee to
     our own stderr / tracing.
4. Awaits ONLY `tokio::time::timeout(deadline, child.wait())` in the parent —
   the timeout thus measures wall-clock process lifespan, unaffected by
   whether the child is emitting output or silent.
5. On any return, calls the process-group kill, then joins the reader tasks
   (SIGKILL closes the child's fds -> readers hit EOF -> join completes
   instantly and yields the collected/partial results).

### Why this beats a single `select!` loop

- **No starvation / silence bug**: a `select!` polling {stdout, stderr, wait,
  timeout} can starve the timeout or stderr arm while stdout is busy (JSON
  parse or `flush().await`). Isolating the timeout around `child.wait()`
  guarantees it always fires — including when the child is alive but silent
  (the investigation's critical edge case).
- **No pipe-buffer deadlock**: stderr is drained by its own task, so a child
  that spews stderr can never block and stall its stdout.
- **Clean teardown = partial-log survival**: we do not `.abort()` readers.
  The group-kill closes fds -> readers reach EOF naturally -> they return the
  lines already collected/logged. On timeout we return an error but keep the
  streamed run-log and partial events.

## Process-group kill machinery (port from cast, do not reinvent)

Source: `crates/cast/src/mcp/exec.rs` (verified this session).

- `ProcessGroupGuard` RAII struct (37-47) + `kill_process_group` (52-65):
  `unsafe { libc::kill(-(pgid as libc::pid_t), libc::SIGKILL) }`; `pgid == 0`
  guarded against; `ESRCH` ignored (group already gone).
- `cmd.process_group(0)` (84) -> child PID == PGID.
- `child.id()` captured immediately after spawn (112) becomes the PGID.
- `kill_on_drop(true)` (87) kept as a zombie-reaper fallback — but note it
  only SIGKILLs the DIRECT child, NOT grandchildren, which is exactly why the
  explicit `killpg` is required.
- Model AC-4 test on `test_timeout_kills_entire_process_tree` (421-488):
  child records its PGID to a tempfile then sleeps; assert `libc::killpg(pgid,
  0) != 0` (ESRCH) after the deadline.

Refinement over exec.rs: give the guard a `kill_now(&mut self)` that
`take()`s the pgid Option so an explicit deterministic kill on timeout does
NOT double-fire on drop (drop becomes a no-op once taken). Drop still covers
the future-cancellation path.

Divergence from exec.rs: exec.rs uses `child.wait_with_output()` (buffered).
We must NOT reuse that output path — we take the pipe handles and stream. Only
the kill/guard machinery is reused.

## Repo facts (verified)

- Tokio 1.52.3 across the workspace. `cast` already enables `process`,
  `signal`, `rt-multi-thread`, `macros`; `time` used implicitly.
- The new crate's tokio features MUST explicitly include: `rt-multi-thread`,
  `macros`, `process`, `signal`, `io-util` (for `BufReader`/`AsyncBufReadExt`
  — currently NOT enabled anywhere), `time`, `sync`, and `fs` IF we use
  `tokio::fs` for the run-log (alternative: a `std::fs::File` +
  `tokio::task::spawn_blocking`, or a synchronous `BufWriter` with explicit
  flush — decide in implementation; `fs` feature is the simplest).
- `libc = "0.2"` already a dependency of `cast` (`crates/cast/Cargo.toml:19`),
  gated `#[cfg(unix)]`. Add the same to cast-agent.
- **No existing streaming/BufReader/mpsc subprocess I/O anywhere in the repo**
  — this is a genuinely new pattern for the codebase.
- Test conventions: route all fs to temp dirs (e.g.
  `crates/cast/tests/cli_test.rs:6-7` sets `CAST_LOG_DIR`/`CAST_DATA_DIR` to
  `std::env::temp_dir().join(...)`); use `tempfile`, `assert_cmd`,
  `predicates`, `tokio-util`; no `$HOME`/network (Nix sandbox).
- Crate template: `cast-mcp-client` layout — `src/main.rs`, `src/lib.rs`,
  `src/commands/`, `src/client/`, `tests/`.

## Post-review updates (authoritative over the sketch below)

The sketch that follows predates the two Opus review passes + the empirical
verification traces. It is kept for its structure, but the following override it
(details + rationale in `plan/1785127643-8ca9cdd/phase-1-mvp.md` "Pre-implementation
findings" and the traces referenced there):

- **Run-log writer**: use a sync `std::fs::BufWriter` on a dedicated blocking
  thread fed over a channel, `flush()` PER LINE — NOT `tokio::fs` (24x slower;
  its `flush().await` is a near no-op — benchmark:
  `trace/.../runlog-writer-flush-benchmark.md`). No tokio `fs` feature.
- **`drop(stdin)` is load-bearing** (verified: opencode blocks on stdin EOF
  before starting) and the prompt write goes in its OWN task (B2c: avoids
  >64KiB deadlock + EPIPE-before-result.json).
- **Inherit parent env** for the harness child (C5) — do NOT `env_clear()`.
- **Bound BOTH reader-task joins** with a `timeout`, falling back to the on-disk
  trace (C1: a grandchild holding the pipe write-end never lets the reader EOF).
- **Re-`wait()`/`try_wait()` after kill** to classify the exit (B2a).
- **Split the guard**: graceful two-phase on interrupt/timeout arms; SIGKILL-only,
  non-blocking `Drop`. Double-signal escalates by `select!`ing signals DURING the
  grace window.
- **No latent panics**: replace `.expect("open run-log")`; persist run-dir /
  prompt.txt / pid BEFORE spawn; write `result.json` atomically (tmp + rename).

## Annotated supervisor sketch (reference — see Post-review updates above)

```rust
// tokio features: rt-multi-thread, macros, process, signal, io-util, time, sync, fs
use std::process::{ExitStatus, Stdio};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use serde_json::Value;

#[cfg(unix)]
struct ProcessGroupGuard(Option<i32>);
#[cfg(unix)]
impl ProcessGroupGuard {
    fn kill_now(&mut self) {
        if let Some(pgid) = self.0.take() {
            unsafe { libc::kill(-pgid, libc::SIGKILL); } // ESRCH ignored
        }
    }
}
#[cfg(unix)]
impl Drop for ProcessGroupGuard {
    fn drop(&mut self) { self.kill_now(); }
}

pub enum RunOutcome {
    Success { status: ExitStatus, events: Vec<Value> },
    Timeout { partial_events: Vec<Value> },
}

pub async fn supervise(
    exe: &str, args: &[String], prompt: &[u8],
    run_log_path: &std::path::Path, deadline: Duration,
) -> std::io::Result<RunOutcome> {
    let mut cmd = Command::new(exe);
    cmd.args(args)
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)] cmd.process_group(0);

    let mut child = cmd.spawn()?;
    #[cfg(unix)]
    let mut guard = ProcessGroupGuard(child.id().map(|p| p as i32));

    // prompt -> stdin -> EOF
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(prompt).await?;
    drop(stdin);

    // stdout reader: live-flush log + collect events
    let stdout = child.stdout.take().unwrap();
    let log_path = run_log_path.to_path_buf();
    let stdout_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        let mut events = Vec::new();
        let mut log = tokio::fs::OpenOptions::new()
            .create(true).append(true).open(&log_path).await
            .expect("open run-log");
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = log.write_all(line.as_bytes()).await;
            let _ = log.write_all(b"\n").await;
            let _ = log.flush().await;                 // live tail -f
            if let Ok(v) = serde_json::from_str::<Value>(&line) { events.push(v); }
        }
        events
    });

    // stderr drain (deadlock prevention)
    let stderr = child.stderr.take().unwrap();
    let stderr_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await { eprintln!("{line}"); }
    });

    // ONLY the wait is under the wall-clock timeout
    let waited = timeout(deadline, child.wait()).await;

    #[cfg(unix)] guard.kill_now();     // deterministic; drop becomes no-op
    let _ = stderr_task.await;
    let events = stdout_task.await.unwrap_or_default();

    match waited {
        Ok(Ok(status)) => Ok(RunOutcome::Success { status, events }),
        Ok(Err(e)) => Err(e),
        Err(_) => Ok(RunOutcome::Timeout { partial_events: events }),
    }
}
```

## Test matrix (Slice 1c acceptance)

Use small `sh`/`printf` scripts as the fake harness; all fs via `tempfile` /
`std::env::temp_dir()` (Nix-safe).

1. **Grandchild-leak / AC-4**: child `sh -c "sleep 100 & sleep 100"` records
   PGID -> timeout -> assert `killpg(pgid, 0)` returns ESRCH (whole group
   dead). Mirrors exec.rs test.
2. **Pipe-buffer deadlock**: child spews multi-MB to stderr, then 1 JSON line
   to stdout, exits -> assert Success with the 1 event (stderr actively
   drained).
3. **Silent hang**: child `sleep 10`, no output -> assert Timeout at ~deadline
   (timeout isolated from silent reader).
4. **Live flush**: child prints 1 JSON line then `sleep 2`; a concurrent
   watcher asserts the line is in the run-log BEFORE `supervise` returns.
5. **Partial-log survival**: child prints 3 JSON lines then `sleep 10` ->
   Timeout with exactly 3 `partial_events`; run-log has 3 lines.
6. **No trailing newline**: child `printf '{"a":1}'` -> event parsed on normal
   exit.
7. **Future-drop cancellation**: wrap `supervise` in an outer
   `tokio::time::timeout` that fires early -> child + grandchildren killed via
   `Drop` (guard covers cancellation).

## Open items rolled forward

- RESOLVED: run-log writer = sync `std::fs::BufWriter` on a blocking thread,
  flush per line (benchmark `trace/.../runlog-writer-flush-benchmark.md`); the
  `tokio::fs` option is rejected.
- Run-log path scheme + reporting it on stderr is settled by the run-dir layout
  (`note/recovery-outcome-contract.md`): `<run-dir>/stream.jsonl`, path printed
  to stderr at startup.
- `extract_result` consumes `events` from `RunOutcome` (harness-specific,
  Slice 1b). On Timeout we still have `partial_events` available if we later
  want best-effort extraction (MVP: Timeout is an error). The opencode
  "last `text` event" rule is CONFIRMED valid (verification trace).
