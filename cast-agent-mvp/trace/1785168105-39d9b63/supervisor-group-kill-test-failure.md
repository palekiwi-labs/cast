# Trace: supervisor `timeout_kills_entire_process_group` failure (handoff)

Status at handoff: Slice 1c supervisor implemented; **8 of 9** supervisor
failure-mode tests pass. One test fails intermittently/consistently and needs
diagnosis before Slice 1c can be called green. The supervisor code itself is
UNCOMMITTED (see "Working tree" below).

## Where things stand

Committed (all green):
- c650dfc Slice 1a scaffold (AC 1)
- dddc8b7 + f0c4177 Slice 1b Harness trait + opencode adapter (AC 2)
- cef4626 prompt resolution; 39d9b63 run-dir resolution

Working tree (UNCOMMITTED, do not lose):
- `crates/cast-agent/src/supervisor.rs` (new) — the reader-task supervisor
- `crates/cast-agent/tests/supervisor_test.rs` (new) — 9-test failure matrix
- `crates/cast-agent/src/lib.rs` (modified) — adds `#[cfg(unix)] pub mod supervisor;`

## The failing test

`tests/supervisor_test.rs::timeout_kills_entire_process_group`

```
let (exe, args) = sh("sleep 100 & sleep 100");
let out = supervise(&exe, &args, b"", &paths(dir.path()), limits(400)).await?;
assert!(matches!(out.end, EndReason::TimedOut));
let pgid = out.pgid.expect("pgid captured");
tokio::time::sleep(Duration::from_millis(150)).await;
assert!(!group_alive(pgid), "process group {pgid} should be dead");   // <-- FAILS
```

`group_alive` = `libc::killpg(pgid, 0) == 0`.

Observed: after the deadline fires and `guard.kill_now()` SIGKILLs the group,
`killpg(pgid, 0)` still returns 0 (group alive) 150ms later. The other 8 tests
(normal completion, no-trailing-newline, silent-hang timeout, partial-log
survival, stderr firehose, stdin-EOF invariant, live-flush, non-zero exit) all
PASS. This is the exec.rs AC-4 analogue (`test_timeout_kills_entire_process_tree`).

## What is known / ruled out

- The port of the kill machinery mirrors `crates/cast/src/mcp/exec.rs:52-65`
  (`kill(-(pgid), SIGKILL)`, pgid==0 guard, ESRCH ignored). exec.rs's own test
  passes in the cast crate, so the mechanism is sound in principle.
- `process_group(0)` is set on the Command; `pgid = child.id()` captured once
  post-spawn (== group leader pid). `out.pgid` returned Some (test printed 3717).
- The bash reproduction attempt was INVALID: running `sh -c '...' &` from bash
  left `sh` in bash's own process group, so `kill -9 -$SHPID` targeted a
  non-leader and gave a misleading ESRCH. Do NOT trust that experiment.

## Leading hypotheses (for the next agent)

1. **Reap ordering / signal race.** In `supervise`, the TimedOut select arm
   calls `guard.kill_now()`, then after the select the code calls
   `guard.kill_now()` again (no-op, disarmed) and then `child.wait().await` to
   reap ONLY the direct child (the shell). The two backgrounded `sleep`
   grandchildren are SIGKILLed via the group signal but are NOT explicitly
   waited/reaped — they may linger as the group-leader pid's group still
   showing "alive" to `killpg(_, 0)` until the kernel fully tears down. 150ms
   may simply be too short, OR the group leader's reap is what keeps the pgid
   slot alive. Try: increase the settle sleep to ~400ms; and/or verify with
   `ps -o pgid` inside the test window.

2. **dash exec-optimization** on `sh -c 'sleep 100 & sleep 100'`: the shell may
   `exec` into the foreground `sleep`, changing which process is the group
   leader vs what `child.id()` reported. Reconfirm the group leader pid is the
   one we signal. Consider the exec.rs-style fake instead: have the shell write
   `$$` to a tempfile and assert on THAT pgid (exec.rs does exactly this at
   lines 427-488), decoupling the assertion from `out.pgid`.

3. **killpg(pgid,0) semantics with a zombie leader.** If the group leader has
   been SIGKILLed but not yet reaped (zombie), `killpg(pgid,0)` can still
   return 0 because the pgid is still allocated. Since we DO reap the direct
   child, the leader shouldn't be a lingering zombie — but a backgrounded
   grandchild that became the de-facto group anchor might. Reaping/among-group
   semantics worth confirming against how exec.rs's passing test structures it.

## Recommended next step

Re-model this test EXACTLY on the proven exec.rs test
(`crates/cast/src/mcp/exec.rs:421-488`): child writes `$$` to a NamedTempFile
then `sleep 100`; after the deadline, read the pgid from the file and assert
`libc::killpg(pgid, 0) != 0` (ESRCH). That test is known-good in the cast crate
with a 400ms deadline + 100ms settle, so matching its shape should either pass
or isolate the real divergence (streaming supervisor vs buffered
`wait_with_output`). If it still fails, the divergence is in the supervisor's
reap/kill ordering, not the kill primitive.

## Remaining Slice 1c work after this is fixed

- Grandchild-holds-pipe bounded-join test (C1) — currently the in-group case is
  covered indirectly; add the explicit escaped-pipe case if practical.
- Interrupt tests (AC 6): spawn cast-agent as a child, send SIGINT/SIGTERM,
  assert `result.json.outcome == interrupted` + dead group. Needs `main.rs`
  wired + `result.json` finalize first. Also double-signal escalation +
  trap-SIGTERM-grace tests.
- `finalize`/`result.json` module (atomic tmp+rename), exit-code mapping,
  run-dir persist-before-spawn (prompt.txt, cast-agent.pid), stderr run-dir path
  print at startup.
- Wire `main.rs run` end-to-end; e2e stream test (AC 3 automated portion).
- Slice 1d: flake.nix `cast-agent` package + docs README; `nix build`/QA smoke.
