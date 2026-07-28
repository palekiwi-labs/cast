//! Supervisor failure-mode matrix. Scripted `sh` fakes; all fs via tempdir
//! (Nix-sandbox safe). Unix-only (cast-agent runs in a Linux container).
#![cfg(unix)]

use cast_agent::supervisor::{EndReason, Limits, RunPaths, supervise};
use std::time::{Duration, Instant};

fn paths(dir: &std::path::Path) -> RunPaths {
    RunPaths {
        stream_jsonl: dir.join("stream.jsonl"),
        stderr_log: dir.join("stderr.log"),
    }
}

fn limits(deadline_ms: u64) -> Limits {
    Limits {
        deadline: Duration::from_millis(deadline_ms),
        grace: Duration::from_millis(300),
    }
}

fn sh(script: &str) -> (String, Vec<String>) {
    ("sh".to_string(), vec!["-c".to_string(), script.to_string()])
}

/// Count the LIVE (non-zombie) members of a process group by parsing `ps`.
///
/// `killpg(pgid, 0)` cannot be used to prove teardown here: after the group is
/// SIGKILLed, backgrounded grandchildren are reparented to pid 1, and in this
/// sandbox/container pid 1 is not a reaping init — the dead processes linger as
/// zombies (state `Z`) indefinitely, so `killpg` keeps returning 0 for
/// already-dead processes. The real requirement is that no *live* process
/// escapes the kill, so we count members whose state is not `Z`/defunct.
fn live_group_members(pgid: i32) -> Vec<String> {
    let out = std::process::Command::new("ps")
        .args(["-eo", "pid=,pgid=,stat=,comm="])
        .output()
        .expect("ps");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let _pid = it.next()?;
            let gid: i32 = it.next()?.parse().ok()?;
            let stat = it.next()?;
            if gid == pgid && !stat.starts_with('Z') {
                Some(line.trim().to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Wait up to `budget` for a process group to have no LIVE members, polling.
async fn wait_group_dead(pgid: i32, budget: Duration) -> bool {
    let start = Instant::now();
    loop {
        if live_group_members(pgid).is_empty() {
            return true;
        }
        if start.elapsed() >= budget {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

#[tokio::test]
async fn normal_completion_collects_events() {
    let dir = tempfile::tempdir().unwrap();
    let (exe, args) = sh(r#"printf '{"type":"text","part":{"text":"hi"}}\n'"#);
    let out = supervise(&exe, &args, b"prompt", &paths(dir.path()), limits(5000))
        .await
        .unwrap();
    assert!(matches!(out.end, EndReason::Exited(s) if s.success()));
    assert_eq!(out.events.len(), 1);
    assert_eq!(out.events[0]["part"]["text"], "hi");
}

#[tokio::test]
async fn no_trailing_newline_still_parses() {
    let dir = tempfile::tempdir().unwrap();
    let (exe, args) = sh(r#"printf '{"a":1}'"#); // no trailing newline
    let out = supervise(&exe, &args, b"", &paths(dir.path()), limits(5000))
        .await
        .unwrap();
    assert_eq!(out.events.len(), 1);
    assert_eq!(out.events[0]["a"], 1);
}

#[tokio::test]
async fn timeout_kills_entire_process_group() {
    let dir = tempfile::tempdir().unwrap();
    // Shell backgrounds a sleep (grandchild) and itself sleeps: whole group
    // must be dead after the deadline.
    let (exe, args) = sh("sleep 100 & sleep 100");
    let out = supervise(&exe, &args, b"", &paths(dir.path()), limits(400))
        .await
        .unwrap();
    assert!(matches!(out.end, EndReason::TimedOut));
    let pgid = out.pgid.expect("pgid captured");
    // The whole group must be torn down. A backgrounded grandchild is
    // reparented to init after we reap the leader and may linger as a zombie
    // for a beat, so poll for eventual death rather than a single instant.
    assert!(
        wait_group_dead(pgid, Duration::from_secs(3)).await,
        "process group {pgid} has live (non-zombie) members: {:?}",
        live_group_members(pgid)
    );
}

#[tokio::test]
async fn silent_child_times_out_at_deadline() {
    let dir = tempfile::tempdir().unwrap();
    let (exe, args) = sh("sleep 100"); // no output at all
    let start = Instant::now();
    let out = supervise(&exe, &args, b"", &paths(dir.path()), limits(400))
        .await
        .unwrap();
    assert!(matches!(out.end, EndReason::TimedOut));
    assert!(
        start.elapsed() < Duration::from_secs(3),
        "timeout must fire"
    );
}

#[tokio::test]
async fn partial_log_survives_timeout() {
    let dir = tempfile::tempdir().unwrap();
    let (exe, args) = sh(r#"printf '{"n":1}\n{"n":2}\n{"n":3}\n'; sleep 100"#);
    let out = supervise(&exe, &args, b"", &paths(dir.path()), limits(500))
        .await
        .unwrap();
    assert!(matches!(out.end, EndReason::TimedOut));
    assert_eq!(out.events.len(), 3, "3 lines emitted before the hang");
    let disk = std::fs::read_to_string(dir.path().join("stream.jsonl")).unwrap();
    assert_eq!(disk.lines().count(), 3);
}

#[tokio::test]
async fn stderr_firehose_does_not_deadlock() {
    let dir = tempfile::tempdir().unwrap();
    // Spew a lot to stderr, then one JSON line to stdout, then exit. If stderr
    // were not actively drained the child would block on a full pipe.
    let (exe, args) = sh(
        r#"i=0; while [ $i -lt 5000 ]; do echo "noise line $i" >&2; i=$((i+1)); done; printf '{"done":1}\n'"#,
    );
    let out = supervise(&exe, &args, b"", &paths(dir.path()), limits(10000))
        .await
        .unwrap();
    assert!(matches!(out.end, EndReason::Exited(s) if s.success()));
    assert_eq!(out.events.len(), 1);
    assert_eq!(out.events[0]["done"], 1);
}

#[tokio::test]
async fn stdin_is_closed_so_child_that_drains_it_completes() {
    let dir = tempfile::tempdir().unwrap();
    // `cat` reads stdin to EOF, then we emit. Only completes if stdin is EOF'd
    // (load-bearing invariant: opencode blocks on stdin EOF before starting).
    let (exe, args) = sh(r#"cat >/dev/null; printf '{"ok":1}\n'"#);
    let out = supervise(&exe, &args, b"the prompt", &paths(dir.path()), limits(5000))
        .await
        .unwrap();
    assert!(matches!(out.end, EndReason::Exited(s) if s.success()));
    assert_eq!(out.events.len(), 1);
}

#[tokio::test]
async fn live_flush_writes_before_supervise_returns() {
    let dir = tempfile::tempdir().unwrap();
    let stream = dir.path().join("stream.jsonl");
    // Emit one line, then sleep well within the deadline.
    let (exe, args) = sh(r#"printf '{"early":1}\n'; sleep 3"#);
    let p = paths(dir.path());
    let handle = tokio::spawn(async move { supervise(&exe, &args, b"", &p, limits(10000)).await });

    // Poll the on-disk log; the line must appear while the child still sleeps
    // (i.e. well before supervise returns ~3s later).
    let start = Instant::now();
    let mut seen = false;
    while start.elapsed() < Duration::from_millis(1500) {
        if let Ok(s) = std::fs::read_to_string(&stream)
            && s.contains("early")
        {
            seen = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        seen,
        "line should be flushed live, before supervise returns"
    );
    assert!(
        !handle.is_finished(),
        "supervise still running when line seen"
    );
    let _ = handle.await.unwrap().unwrap();
}

#[tokio::test]
async fn escaped_grandchild_does_not_hang_supervise() {
    // A grandchild `setsid`s out of the process group while holding the stdout
    // pipe write-end open, then the direct child exits 0. The group SIGKILL
    // cannot reach the escaped grandchild, so the stdout reader never hits EOF.
    // supervise() must still return promptly (bounded reader join + on-disk
    // fallback) rather than hang forever on the writer-thread join.
    let dir = tempfile::tempdir().unwrap();
    // The grandchild `setsid`s into its own session (escaping the process
    // group) and holds fd 1 (the stdout pipe write-end) for the whole sleep;
    // its two-statement body avoids the shell's single-command exec-optimize.
    // The direct child sleeps briefly before exiting so the grandchild's
    // setsid() completes and firmly escapes BEFORE the group SIGKILL fires —
    // otherwise our own kill races and reaps it, masking the bug.
    let (exe, args) =
        sh(r#"printf '{"n":1}\n'; setsid sh -c 'sleep 30; echo x' & sleep 0.3; exit 0"#);
    let p = paths(dir.path());
    let fut = supervise(&exe, &args, b"", &p, limits(10000));
    let out = tokio::time::timeout(Duration::from_secs(8), fut)
        .await
        .expect("supervise must not hang on an escaped grandchild")
        .unwrap();
    assert!(matches!(out.end, EndReason::Exited(s) if s.success()));
    // The single event is recovered via the on-disk stream.jsonl fallback
    // (the in-memory reader task was abandoned because it never EOF'd).
    assert_eq!(out.events.len(), 1);
    assert_eq!(out.events[0]["n"], 1);
}

#[tokio::test]
async fn nonzero_exit_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let (exe, args) = sh("exit 7");
    let out = supervise(&exe, &args, b"", &paths(dir.path()), limits(5000))
        .await
        .unwrap();
    match out.end {
        EndReason::Exited(status) => assert_eq!(status.code(), Some(7)),
        other => panic!("expected Exited(7), got {other:?}"),
    }
}
