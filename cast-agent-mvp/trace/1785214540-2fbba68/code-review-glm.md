# Code Review (diff-reviewer-glm) — feat/cast-agent-mvp

Reviewer: diff-reviewer-glm
Branch: feat/cast-agent-mvp @ 2fbba68 vs merge-base 8ca9cdd
Diff: .cue/cast-agent-mvp/tmp/1785214540-2fbba68/branch.diff

## Verdict

Well-structured; most carefully-designed invariants hold in code. ONE must-fix before merge (B1). I reproduced B1 out-of-tree with a standalone tokio program: a setsid sh -c 'sleep 30' & grandchild holding stdout -> reader join times out and falls back correctly -> writer join() hung past an 8s watchdog that had to kill the process.

## Verified invariants (holding)

process_group(0)+kill_on_drop(true) (supervisor.rs:175-176), env inherited no env_clear (supervisor.rs:177-178), stdin-EOF-in-own-task (supervisor.rs:197-202), dedicated reader tasks, cheap-future-only control plane, single PGID capture (supervisor.rs:184), two-phase graceful teardown + non-blocking Drop backstop, B2a re-wait (supervisor.rs:258-266), persist-before-spawn (run.rs:65-66), atomic result.json (finalize.rs:158-164), correct exit-code table.

## Blocking issues

### B1 — Writer-thread join hangs forever in the C1 escaped-grandchild scenario (defeats C1 + recovery contract) · critical

crates/cast-agent/src/supervisor.rs:268-283

Reader-task joins are correctly bounded (lines 270, 274), but the subsequent writer-thread join is UNBOUNDED:

```rust
// 270: bounded reader join (correct)
let events = match tokio::time::timeout(READER_JOIN_TIMEOUT, stdout_task).await {
    Ok(Ok(events)) => events,
    _ => events_from_disk(&paths.stream_jsonl),
};
let _ = tokio::time::timeout(READER_JOIN_TIMEOUT, stderr_task).await;

// 279-283: UNBOUNDED writer join — hangs forever if a reader was abandoned
let _ = tokio::task::spawn_blocking(move || {
    let _ = stdout_writer.join();
    let _ = stderr_writer.join();
})
.await;
```

Why it hangs. When the stdout reader join times out (the exact C1 case: a setsid/double-forked grandchild holds the stdout pipe write-end open after the group is SIGKILLed), tokio::time::timeout drops the JoinHandle — but dropping a tokio JoinHandle DETACHES the task, it does NOT cancel it. The reader task keeps running, blocked on lines.next_line(), and still owns stdout_tx. The writer thread is parked in while let Ok(line) = rx.recv() (supervisor.rs:128); std::sync::mpsc::RecvError is only returned when ALL senders drop. Since the abandoned task holds a sender that never drops, rx.recv() blocks forever, the writer thread never exits, stdout_writer.join() never returns, the spawn_blocking future never resolves, and supervise() never returns.

Impact. supervise() hanging means orchestrate() never reaches write_result_json (run.rs:94), so result.json is never written and cast-agent hangs indefinitely. Direct violation of both C1 and the core recovery contract ("every run yields a result.json verdict"). The code comment at supervisor.rs:276-278 even asserts "Do not block indefinitely if a reader was abandoned above" — the assertion is FALSE.

The scenario is realistic for this product: agent harnesses spawn tool subprocesses, and any tool that double-forks/setsids (background servers, daemonizing helpers) escapes the process group while inheriting the pipe write-end.

Reproduced with a standalone tokio program mirroring the supervisor: a setsid sh -c 'sleep 30' & grandchild holding stdout -> reader join times out and falls back correctly -> writer join() hung past an 8s watchdog.

Why no test caught it. Every supervisor/interrupt test keeps the grandchild IN-GROUP (sleep 100 & sleep 100, supervisor_test.rs:97), so group SIGKILL reaches it, the pipe closes, the reader EOFs within the 2s budget, and the writer join succeeds. No test exercises an ESCAPED grandchild.

Fix (review-only, two options). Either bound the writer join symmetrically:

```rust
let _ = tokio::time::timeout(READER_JOIN_TIMEOUT, tokio::task::spawn_blocking(move || {
    let _ = stdout_writer.join();
    let _ = stderr_writer.join();
})).await;
```

or — cleaner, matching the stated "non-blocking backstop" philosophy — simply drop the writer JoinHandles without joining. The writer threads already flush per line (supervisor.rs:138), so no durability is lost; a detached writer thread finishes naturally if the reader ever EOFs, and is reaped at process exit:

```rust
drop(stdout_writer);
drop(stderr_writer);
```

Either way, add a regression test that spawns a setsid grandchild holding the pipe and asserts supervise() returns within a few seconds (would also be the first test to cover the events_from_disk fallback).

## Important issues

### I1 — supervise/orchestrate error path writes no verdict and misclassifies as usage error · high

crates/cast-agent/src/supervisor.rs:237, crates/cast-agent/src/run.rs:74, crates/cast-agent/src/main.rs:118-121

Control plane's child-exit arm propagates wait failures with ?:

```rust
res = child.wait() => EndReason::Exited(res?),   // supervisor.rs:237
```

If child.wait() returns Err (I/O error during reap, or kill_on_drop's internal kill failing), ? propagates out of supervise, then out of orchestrate (run.rs:74), then main prints "run failed" and exits with EXIT_USAGE (2) — and NO result.json is written. The recovery contract says every run yields a verdict; this path silently drops it. It also conflates a runtime I/O failure with a usage error (exit 2 = "bad args / unreadable prompt").

More generally, ANY Err out of supervise — including the most realistic one, opencode not on PATH (cmd.spawn()?, supervisor.rs:180) — takes this same path: no verdict, exit 2. A missing harness binary is not a usage error and arguably deserves a failed/crashed verdict so the calling agent gets a structured receipt.

Fix direction. Don't ?-propagate out of the supervisor for a wait failure; map it to an Outcome (e.g. Crashed) and still write result.json. Consider giving spawn/IO failures a distinct non-usage outcome. At minimum, main should write a best-effort verdict before exiting on the Err arm so the "log exists but no result.json => hard-killed" heuristic (finalize.rs:154-157) isn't triggered by ordinary runtime errors.

### I2 — Reader-task death is not observable in the control plane · medium

crates/cast-agent/src/supervisor.rs:236-253

Control-plane select! polls only child.wait(), deadline, sigint, sigterm — NOT the stdout_task/stderr_task handles. If a reader task panics or is aborted mid-run, the control plane never learns directly; it only notices indirectly when the child eventually exits, the deadline fires, or a signal arrives. The reader JoinHandles are only awaited AFTER the control plane has already resolved (supervisor.rs:270, 274).

Real-world impact currently low because reader bodies are panic-free (the only expects are pre-task, supervisor.rs:199/205/221), and tokio catches task panics so cast-agent doesn't crash. But the stated invariant does not hold: a reader dying on a malformed/future opencode stream shape would silently lose events while the run continues.

Fix direction. Add the stdout reader handle as a control-plane arm, or document explicitly why the indirect observability via child-exit is acceptable for MVP.

## Nits / suggestions

- N1 — EndReason::child_signal is dead code (supervisor.rs:311-316). finalize uses status.signal() directly (finalize.rs:107). Remove.
- N2 — tracing and tracing-subscriber declared but UNUSED (Cargo.toml:68-69). No tracing:: macros in src/, main never installs a subscriber. Either wire tracing up or drop both deps.
- N3 — extract_result won't fall back past a malformed last text event (opencode.rs:41-50). .find(...).and_then(text_of_event) returns None if the last type=="text" event has neither part.text nor top-level text, without trying earlier text events. A defensive events.iter().rev().filter(...).filter_map(text_of_event).next() would be more robust.
- N4 — Test coverage gap: the C1 path (escaped grandchild + on-disk fallback) entirely untested. This is why B1 survived. timeout_kills_entire_process_group and interrupt tests all keep grandchildren in-group. A setsid grandchild regression test would cover all three and catch B1 immediately. Strongly recommended alongside the B1 fix.
- N5 — Interrupted records the WRONG thing in exit.signal (finalize.rs:60-61, 119-125). ExitInfo documented as "child's exit disposition: code XOR signal." For TimedOut, exit.signal=SIGKILL (correct, reflects child death). For Interrupted, exit.signal is set to the SIGINT/SIGTERM cast-agent RECEIVED — not the child's actual terminating signal (our post-grace SIGKILL, or clean exit if child honored SIGTERM). Inconsistent; a recovery reader would misinfer the child's death cause. Either rename/document the field as "interrupt cause" or record the actual child disposition.
- N6 — Answer to brief's question: live_group_members (ps-based) workaround is SOUND (supervisor_test.rs:26-53, interrupt_test.rs:13-42). Filtering out zombies asserts the real requirement "no LIVE process escapes the kill." Does NOT mask a production bug. Operational caveat: deployments should run a reaping init (tini/dumb-init) as pid 1 to avoid zombie buildup from escaped grandchildren.
- N7 — Answer to brief's question: PATH-shim test mechanism is SOUND (interrupt_test.rs:104-129). Exercises real production path (Command::new("opencode") -> execvp -> exec shim), no in-process override. CAST_AGENT_FAKE_CMD fully gone. $$-as-pgid reasoning holds. Prepending PATH correct.
- N8 — child_trapping_sigterm_is_sigkilled_after_grace: the >= 3s assertion is valid but for a non-obvious reason worth a one-line comment. trap '' TERM sets SIGTERM to SIG_IGN in sh; IGNORED-signal dispositions are inherited across fork and preserved across execve, so the external sleep child ALSO ignores SIGTERM and survives the group SIGTERM — only the post-grace SIGKILL stops it. A comment would prevent future confusion and protect the test from being "simplified" into a trap-less version.

## Summary

Implementation is well-structured and most carefully-designed invariants hold. The one must-fix before merge is B1: the unbounded writer-thread join (supervisor.rs:279-283) hangs supervise() forever precisely in the C1 escaped-grandchild scenario the code claims to handle, defeating both the C1 fallback and the recovery contract's "always write result.json" guarantee. Reproduced out-of-tree. I1 (error path drops verdict + misuses exit 2) and I2 (reader death invisible to control plane) are the next-most-important gaps. Remaining items are nits and two answered questions confirming the test-environment workarounds (live_group_members, PATH shim) are sound.

Recommended pre-merge actions: fix B1, add an escaped-grandchild regression test (covers B1 + never-tested events_from_disk fallback), decide on I1 verdict-on-error policy.
