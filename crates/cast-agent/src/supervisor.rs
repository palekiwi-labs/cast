//! Streaming process supervisor (dedicated reader-task / actor pattern).
//!
//! Design: `.cue/cast-agent-mvp/doc/streaming-supervisor-design.md` and
//! `.cue/cast-agent-mvp/note/recovery-outcome-contract.md`.
//!
//! The child runs in its own process group with stdin/stdout/stderr piped.
//! The prompt is written to stdin in its own task then EOF'd (load-bearing:
//! opencode blocks on stdin EOF before starting). One reader task drains
//! stdout (live-flushing each line to `stream.jsonl` and collecting parsed
//! events); one drains stderr to `stderr.log`. The parent `select!` is a pure
//! control plane over cheap futures — `child.wait()`, a wall-clock timeout,
//! and the SIGINT/SIGTERM streams — so the timeout fires even when the child
//! is alive but silent. Any arm runs the same process-group teardown.

use anyhow::Result;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Stdio};
use std::sync::mpsc as std_mpsc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

/// How the supervised run ended, before harness-specific classification.
#[derive(Debug)]
pub enum EndReason {
    /// The child exited on its own (possibly non-zero, possibly by its own
    /// crash signal — inspect the status).
    Exited(ExitStatus),
    /// The wall-clock deadline fired; the group was killed. `child_status` is
    /// the child's reaped disposition (how it actually died — normally our
    /// SIGKILL), or `None` if it could not be reaped.
    TimedOut { child_status: Option<ExitStatus> },
    /// cast-agent received SIGINT/SIGTERM (`trigger`); the group was torn down.
    /// `child_status` is how the CHILD actually died (SIGTERM if it honored the
    /// graceful stop, SIGKILL if it had to be force-killed after the grace),
    /// distinct from the `trigger` cast-agent received.
    Interrupted {
        trigger: i32,
        child_status: Option<ExitStatus>,
    },
    /// The harness child could not be spawned (e.g. missing binary). This is a
    /// runtime failure, not a usage error: it still yields a `Crashed` verdict.
    SpawnFailed(String),
    /// Supervision failed after spawn (e.g. an I/O error awaiting the child).
    SuperviseFailed(String),
}

/// The raw product of a supervised run. Harness-agnostic; the orchestrator
/// applies `extract_result` and maps this to a final `RunOutcome`.
#[derive(Debug)]
pub struct SuperviseOutput {
    pub events: Vec<Value>,
    pub end: EndReason,
    pub pgid: Option<i32>,
}

/// Timing limits for a supervised run.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Wall-clock deadline before the group is SIGKILLed.
    pub deadline: Duration,
    /// Grace window between SIGTERM and SIGKILL on the signal path.
    pub grace: Duration,
}

/// On-disk destinations for the two child streams within the run directory.
pub struct RunPaths {
    pub stream_jsonl: PathBuf,
    pub stderr_log: PathBuf,
}

const READER_JOIN_TIMEOUT: Duration = Duration::from_secs(2);

/// RAII guard that kills the child's process group. `kill_now` SIGKILLs and
/// disarms (so `Drop` does not double-fire); `terminate` sends SIGTERM without
/// disarming. `Drop` is SIGKILL-only and non-blocking (never sleeps on a
/// runtime worker) — it backstops the future-cancellation path.
#[cfg(unix)]
pub struct ProcessGroupGuard(Option<i32>);

#[cfg(unix)]
impl ProcessGroupGuard {
    fn new(pgid: Option<i32>) -> Self {
        Self(pgid)
    }

    /// Send SIGTERM to the group (graceful phase). Does not disarm.
    fn terminate(&self) {
        if let Some(pgid) = self.0 {
            send_signal(pgid, libc::SIGTERM);
        }
    }

    /// Send SIGKILL to the group and disarm so `Drop` becomes a no-op.
    fn kill_now(&mut self) {
        if let Some(pgid) = self.0.take() {
            send_signal(pgid, libc::SIGKILL);
        }
    }
}

#[cfg(unix)]
impl Drop for ProcessGroupGuard {
    fn drop(&mut self) {
        self.kill_now();
    }
}

/// `kill(-pgid, sig)` — the POSIX "send to process group" form. `pgid == 0` is
/// refused (would target our own group); `ESRCH` (group already gone) is
/// ignored.
#[cfg(unix)]
fn send_signal(pgid: i32, sig: libc::c_int) {
    if pgid == 0 {
        return;
    }
    // SAFETY: kill() is always safe to call; a negative pid targets the group.
    let ret = unsafe { libc::kill(-(pgid as libc::pid_t), sig) };
    if ret != 0 {
        let e = std::io::Error::last_os_error();
        if e.raw_os_error() != Some(libc::ESRCH) {
            eprintln!("cast-agent: kill(-{pgid}, {sig}): {e}");
        }
    }
}

/// Spawn a dedicated OS thread that owns a per-line-flushing `BufWriter` over
/// `path`, fed via a channel. Flushing per line is the live `tail -f` /
/// crash-durability substrate (benchmark-backed decision — no `tokio::fs`).
fn spawn_line_writer(
    path: &Path,
) -> Result<(std_mpsc::Sender<String>, std::thread::JoinHandle<()>)> {
    use std::io::{BufWriter, Write};
    let file = std::fs::File::create(path)?;
    let (tx, rx) = std_mpsc::channel::<String>();
    let handle = std::thread::spawn(move || {
        let mut w = BufWriter::new(file);
        while let Ok(line) = rx.recv() {
            // Each `line` is written verbatim with a trailing newline, then
            // flushed so a concurrent tailer / a post-SIGKILL reader sees it.
            if w.write_all(line.as_bytes()).is_err() {
                break;
            }
            if w.write_all(b"\n").is_err() {
                break;
            }
            let _ = w.flush();
        }
        let _ = w.flush();
    });
    Ok((tx, handle))
}

/// Parse a run's `stream.jsonl` back into events (fallback when a reader task
/// join is abandoned because a grandchild is holding the pipe write-end open).
fn events_from_disk(path: &Path) -> Vec<Value> {
    match std::fs::read_to_string(path) {
        Ok(s) => s
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str::<Value>(l).ok())
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Supervise `exe args` as a process-isolated child, streaming its output to
/// the run directory and enforcing the wall-clock deadline and signals.
#[cfg(unix)]
pub async fn supervise(
    exe: &str,
    args: &[String],
    prompt: &[u8],
    paths: &RunPaths,
    limits: Limits,
) -> Result<SuperviseOutput> {
    use tokio::signal::unix::{SignalKind, signal};

    let mut cmd = Command::new(exe);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .kill_on_drop(true);
    // C5: inherit the parent env (do NOT env_clear) — the container is the
    // sandbox; clearing would drop HOME / API keys and fail auth.

    // A spawn failure (missing/unexecutable harness binary) is a runtime
    // failure, not a fatal error: yield a Crashed verdict so the recovery
    // contract's "every run yields a receipt" guarantee holds.
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return Ok(SuperviseOutput {
                events: Vec::new(),
                end: EndReason::SpawnFailed(format!("spawn {exe}: {e}")),
                pgid: None,
            });
        }
    };

    // Capture the PGID ONCE, immediately post-spawn, as a plain i32 (never
    // re-read child.id() — it returns None after reap: PID-reuse hazard).
    let pgid = child.id().map(|p| p as i32);
    let mut guard = ProcessGroupGuard::new(pgid);

    // Register both signal streams BEFORE the child work begins so a signal
    // during the run (or during the grace window) is never missed. Installing
    // these also replaces the default terminate disposition so cast-agent
    // controls its own exit.
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigterm = signal(SignalKind::terminate())?;

    // B2c: write the prompt to stdin in its OWN task, then drop it (EOF).
    let mut stdin = child.stdin.take().expect("stdin piped");
    let prompt_owned = prompt.to_vec();
    tokio::spawn(async move {
        let _ = stdin.write_all(&prompt_owned).await;
        // Drop closes the fd -> EOF. Load-bearing: opencode blocks on stdin
        // EOF before starting.
        drop(stdin);
    });

    // stdout reader: live-flush each line + collect parsed events.
    let stdout = child.stdout.take().expect("stdout piped");
    let (stdout_tx, stdout_writer) = spawn_line_writer(&paths.stream_jsonl)?;
    let stdout_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        let mut events = Vec::new();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = stdout_tx.send(line.clone());
            if let Ok(v) = serde_json::from_str::<Value>(&line) {
                events.push(v);
            }
        }
        drop(stdout_tx); // let the writer thread flush and exit
        events
    });

    // stderr reader: drain to stderr.log (prevents pipe-buffer deadlock).
    let stderr = child.stderr.take().expect("stderr piped");
    let (stderr_tx, stderr_writer) = spawn_line_writer(&paths.stderr_log)?;
    let stderr_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = stderr_tx.send(line);
        }
        drop(stderr_tx);
    });

    // Control plane: cheap futures only. The timeout fires even if the child
    // is alive but silent.
    let end = {
        let sleep = tokio::time::sleep(limits.deadline);
        tokio::pin!(sleep);
        tokio::select! {
            res = child.wait() => EndReason::Exited(res?),
            _ = &mut sleep => {
                guard.kill_now();
                EndReason::TimedOut { child_status: None }
            }
            _ = sigint.recv() => {
                graceful_teardown(&mut guard, &mut child, &mut sigint,
                    &mut sigterm, limits.grace).await;
                EndReason::Interrupted { trigger: libc::SIGINT, child_status: None }
            }
            _ = sigterm.recv() => {
                graceful_teardown(&mut guard, &mut child, &mut sigint,
                    &mut sigterm, limits.grace).await;
                EndReason::Interrupted { trigger: libc::SIGTERM, child_status: None }
            }
        }
    };

    // Ensure the group is dead on every path, then reap (B2a) so a TimedOut /
    // Interrupted classification does not leave the status unclassifiable.
    guard.kill_now();
    let end = match end {
        EndReason::TimedOut { .. } => {
            // Re-wait to reap AND to record how the child actually died (our
            // SIGKILL), rather than discarding the status.
            let child_status = child.wait().await.ok();
            EndReason::TimedOut { child_status }
        }
        EndReason::Interrupted { trigger, .. } => {
            // The reaped status reflects the CHILD's real death cause (SIGTERM
            // if it honored the grace, SIGKILL otherwise) — distinct from the
            // `trigger` signal cast-agent received.
            let child_status = child.wait().await.ok();
            EndReason::Interrupted {
                trigger,
                child_status,
            }
        }
        // Exited / SpawnFailed / SuperviseFailed need no reap here.
        other => other,
    };

    // C1 + B1: bound BOTH reader joins. A grandchild that holds a pipe
    // write-end open (e.g. a `setsid` daemon escaping the group SIGKILL) means
    // the reader never hits EOF. On timeout we must ABORT the reader task, not
    // merely drop its JoinHandle: dropping DETACHES the task, leaving it
    // running and still owning its line-writer `Sender`, which keeps the writer
    // thread's blocking `recv()` — and thus the writer join below — hung
    // forever. Aborting drops the abandoned task's `Sender`, releasing the
    // writer. Fall back to the on-disk trace for the events.
    let mut stdout_task = stdout_task;
    let events = tokio::select! {
        joined = &mut stdout_task => joined.unwrap_or_default(),
        _ = tokio::time::sleep(READER_JOIN_TIMEOUT) => {
            stdout_task.abort();
            let _ = stdout_task.await; // await the cancellation -> drop Sender
            events_from_disk(&paths.stream_jsonl)
        }
    };

    let mut stderr_task = stderr_task;
    tokio::select! {
        _ = &mut stderr_task => {}
        _ = tokio::time::sleep(READER_JOIN_TIMEOUT) => {
            stderr_task.abort();
            let _ = stderr_task.await;
        }
    }

    // Writer threads exit once their `Sender`s drop (the readers EOF'd or were
    // aborted above). Bound the join defensively so a wedged writer can never
    // hang supervise() — durability already comes from the per-line flush.
    let _ = tokio::time::timeout(
        READER_JOIN_TIMEOUT,
        tokio::task::spawn_blocking(move || {
            let _ = stdout_writer.join();
            let _ = stderr_writer.join();
        }),
    )
    .await;

    Ok(SuperviseOutput { events, end, pgid })
}

/// Two-phase graceful stop: SIGTERM the group, wait the grace window, then
/// SIGKILL. A second signal (or the child exiting) DURING the grace window
/// short-circuits to the SIGKILL immediately.
#[cfg(unix)]
async fn graceful_teardown(
    guard: &mut ProcessGroupGuard,
    child: &mut tokio::process::Child,
    sigint: &mut tokio::signal::unix::Signal,
    sigterm: &mut tokio::signal::unix::Signal,
    grace: Duration,
) {
    guard.terminate();
    let sleep = tokio::time::sleep(grace);
    tokio::pin!(sleep);
    tokio::select! {
        _ = child.wait() => {}      // exited within grace
        _ = &mut sleep => {}        // grace expired
        _ = sigint.recv() => {}     // double-signal escalates
        _ = sigterm.recv() => {}
    }
    guard.kill_now();
}

impl EndReason {
    /// The signal number the child was terminated by, if it died by signal.
    #[cfg(unix)]
    pub fn child_signal(status: &ExitStatus) -> Option<i32> {
        status.signal()
    }
}
