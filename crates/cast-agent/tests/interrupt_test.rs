//! AC 6: signal-driven interruption. Spawn the real `cast-agent` binary
//! running a scripted fake harness (via `CAST_AGENT_FAKE_CMD`), signal it
//! mid-run, and assert it writes `result.json(outcome=interrupted)` and tears
//! down the child's process group (no live orphans). All fs via tempdir.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_cast-agent");

/// Count LIVE (non-zombie) members of a process group via `ps`. `killpg(_,0)`
/// is unusable here: this sandbox's pid 1 does not reap orphaned grandchildren,
/// so SIGKILLed processes linger as zombies (see supervisor_test).
fn live_group_members(pgid: i32) -> Vec<String> {
    let out = Command::new("ps")
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

fn wait_group_dead(pgid: i32, budget: Duration) -> bool {
    let start = Instant::now();
    loop {
        if live_group_members(pgid).is_empty() {
            return true;
        }
        if start.elapsed() >= budget {
            return false;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn wait_for_file(path: &Path, budget: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < budget {
        if path.exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    path.exists()
}

/// The single run subdir created under `base`.
fn find_run_dir(base: &Path) -> PathBuf {
    std::fs::read_dir(base)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("a run dir under the base")
}

/// Spawn cast-agent running `script` as its fake harness, with the run-dir
/// base pinned to `base`. Returns the child handle.
fn spawn_agent(base: &Path, script: &str) -> Child {
    Command::new(BIN)
        .args([
            "run",
            "--harness",
            "opencode",
            "--timeout",
            "60",
            "the prompt",
        ])
        .env("CAST_AGENT_FAKE_CMD", script)
        .env("CAST_AGENT_RUN_DIR", base)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn cast-agent")
}

fn kill(pid: u32, sig: i32) {
    unsafe {
        libc::kill(pid as libc::pid_t, sig);
    }
}

fn read_result(base: &Path) -> serde_json::Value {
    let rj = find_run_dir(base).join("result.json");
    serde_json::from_str(&std::fs::read_to_string(rj).unwrap()).unwrap()
}

/// Fake that records its own PGID to `pgid_file`, emits one line, then hangs.
fn hang_script(pgid_file: &Path) -> String {
    format!(
        "printf '%d' $$ > {p}; \
         printf '{{\"type\":\"text\",\"part\":{{\"text\":\"working\"}}}}\\n'; \
         sleep 100",
        p = pgid_file.display()
    )
}

fn assert_interrupted(base: &Path, pgid_file: &Path, signal: i32) {
    let pgid: i32 = std::fs::read_to_string(pgid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(
        wait_group_dead(pgid, Duration::from_secs(5)),
        "child group {pgid} has live members: {:?}",
        live_group_members(pgid)
    );
    let v = read_result(base);
    assert_eq!(v["outcome"], "interrupted");
    assert_eq!(v["exit"]["signal"], signal);
}

#[test]
fn sigint_interrupts_and_tears_down_group() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("runs");
    std::fs::create_dir_all(&base).unwrap();
    let pgid_file = dir.path().join("child.pgid");

    let mut agent = spawn_agent(&base, &hang_script(&pgid_file));
    assert!(
        wait_for_file(&pgid_file, Duration::from_secs(5)),
        "child started"
    );
    std::thread::sleep(Duration::from_millis(100));

    kill(agent.id(), libc::SIGINT);
    let status = agent.wait().unwrap();
    assert_eq!(status.code(), Some(4), "interrupted exit code");
    assert_interrupted(&base, &pgid_file, libc::SIGINT);
}

#[test]
fn sigterm_interrupts_and_tears_down_group() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("runs");
    std::fs::create_dir_all(&base).unwrap();
    let pgid_file = dir.path().join("child.pgid");

    let mut agent = spawn_agent(&base, &hang_script(&pgid_file));
    assert!(
        wait_for_file(&pgid_file, Duration::from_secs(5)),
        "child started"
    );
    std::thread::sleep(Duration::from_millis(100));

    kill(agent.id(), libc::SIGTERM);
    let status = agent.wait().unwrap();
    assert_eq!(status.code(), Some(4), "interrupted exit code");
    assert_interrupted(&base, &pgid_file, libc::SIGTERM);
}

#[test]
fn child_trapping_sigterm_is_sigkilled_after_grace() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("runs");
    std::fs::create_dir_all(&base).unwrap();
    let pgid_file = dir.path().join("child.pgid");

    // The child ignores SIGTERM; only SIGKILL after the 3s grace can stop it.
    let script = format!(
        "trap '' TERM; printf '%d' $$ > {p}; \
         printf '{{\"type\":\"text\",\"part\":{{\"text\":\"stubborn\"}}}}\\n'; \
         sleep 100",
        p = pgid_file.display()
    );

    let mut agent = spawn_agent(&base, &script);
    assert!(
        wait_for_file(&pgid_file, Duration::from_secs(5)),
        "child started"
    );
    std::thread::sleep(Duration::from_millis(100));

    let start = Instant::now();
    kill(agent.id(), libc::SIGINT);
    let status = agent.wait().unwrap();
    // The graceful window must elapse before the SIGKILL lands.
    assert!(
        start.elapsed() >= Duration::from_secs(3),
        "should wait the grace window before SIGKILL"
    );
    assert_eq!(status.code(), Some(4));
    assert_interrupted(&base, &pgid_file, libc::SIGINT);
}
