//! AC 6: signal-driven interruption. Spawn the real `cast-agent` binary
//! running a scripted fake harness, signal it mid-run, and assert it writes
//! `result.json(outcome=interrupted)` and tears down the child's process
//! group (no live orphans). The fake harness is a `#!/bin/sh` shim named
//! `opencode` placed first on the spawned child's `PATH`, so the production
//! spawn path (`harness.base_command()` -> PATH lookup -> exec) runs
//! end-to-end. All fs via tempdir.
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

/// Write a `#!/bin/sh` shim named `opencode` into `shim_dir` with `0o755`
/// perms, whose body is `script`. The kernel execs the file directly, so
/// `$$` inside `script` is the PID cast-agent spawns into its new process
/// group — i.e. the same PID the supervisor will signal and the test records
/// to `pgid_file`. Do NOT `exec` into a subshell from inside the shim, or
/// `$$` and the signaled PID diverge.
fn write_opencode_shim(shim_dir: &Path, script: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(shim_dir).unwrap();
    let shim = shim_dir.join("opencode");
    std::fs::write(&shim, format!("#!/bin/sh\n{script}\n")).unwrap();
    let perms = std::fs::Permissions::from_mode(0o755);
    std::fs::set_permissions(&shim, perms).unwrap();
}

/// Spawn cast-agent with `opencode` on PATH resolving to a shim running
/// `script`, and the run-dir base pinned to `base`. Prepends (does not
/// replace) PATH so the shim itself can still resolve `sh`/`printf`/`sleep`.
fn spawn_agent(base: &Path, shim_dir: &Path, script: &str) -> Child {
    write_opencode_shim(shim_dir, script);
    let mut new_path = shim_dir.as_os_str().to_owned();
    new_path.push(":");
    if let Some(orig) = std::env::var_os("PATH") {
        new_path.push(&orig);
    }
    Command::new(BIN)
        .args([
            "run",
            "--harness",
            "opencode",
            "--timeout",
            "60",
            "the prompt",
        ])
        .env("PATH", &new_path)
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

/// `trigger` is the signal cast-agent received; `child_death` is how the child
/// actually died (SIGTERM if it honored the graceful stop, SIGKILL if it had to
/// be force-killed after the grace). `exit.signal` reflects `child_death`;
/// `interrupt_signal` reflects `trigger`.
fn assert_interrupted(base: &Path, pgid_file: &Path, trigger: i32, child_death: i32) {
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
    assert_eq!(v["exit"]["signal"], child_death);
    assert_eq!(v["interrupt_signal"], trigger);
}

#[test]
fn sigint_interrupts_and_tears_down_group() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("runs");
    std::fs::create_dir_all(&base).unwrap();
    let shim_dir = dir.path().join("shim");
    let pgid_file = dir.path().join("child.pgid");

    let mut agent = spawn_agent(&base, &shim_dir, &hang_script(&pgid_file));
    assert!(
        wait_for_file(&pgid_file, Duration::from_secs(5)),
        "child started"
    );
    std::thread::sleep(Duration::from_millis(100));

    kill(agent.id(), libc::SIGINT);
    let status = agent.wait().unwrap();
    assert_eq!(status.code(), Some(4), "interrupted exit code");
    // Trigger SIGINT; the child (plain `sleep`) honors the graceful SIGTERM.
    assert_interrupted(&base, &pgid_file, libc::SIGINT, libc::SIGTERM);
}

#[test]
fn sigterm_interrupts_and_tears_down_group() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("runs");
    std::fs::create_dir_all(&base).unwrap();
    let shim_dir = dir.path().join("shim");
    let pgid_file = dir.path().join("child.pgid");

    let mut agent = spawn_agent(&base, &shim_dir, &hang_script(&pgid_file));
    assert!(
        wait_for_file(&pgid_file, Duration::from_secs(5)),
        "child started"
    );
    std::thread::sleep(Duration::from_millis(100));

    kill(agent.id(), libc::SIGTERM);
    let status = agent.wait().unwrap();
    assert_eq!(status.code(), Some(4), "interrupted exit code");
    // Trigger SIGTERM; the child (plain `sleep`) dies by the graceful SIGTERM.
    assert_interrupted(&base, &pgid_file, libc::SIGTERM, libc::SIGTERM);
}

#[test]
fn child_trapping_sigterm_is_sigkilled_after_grace() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("runs");
    std::fs::create_dir_all(&base).unwrap();
    let shim_dir = dir.path().join("shim");
    let pgid_file = dir.path().join("child.pgid");

    // The child ignores SIGTERM; only SIGKILL after the 3s grace can stop it.
    // `trap '' TERM` sets SIGTERM to the ignored disposition, which is INHERITED
    // across fork/execve — so this reliably models a real harness (or any
    // grandchild) that survives the graceful phase and forces the SIGKILL
    // escalation path.
    let script = format!(
        "trap '' TERM; printf '%d' $$ > {p}; \
         printf '{{\"type\":\"text\",\"part\":{{\"text\":\"stubborn\"}}}}\\n'; \
         sleep 100",
        p = pgid_file.display()
    );

    let mut agent = spawn_agent(&base, &shim_dir, &script);
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
    // Trigger SIGINT; the child traps/ignores SIGTERM so it is force-killed by
    // SIGKILL after the grace window.
    assert_interrupted(&base, &pgid_file, libc::SIGINT, libc::SIGKILL);
}
