# Code Review (diff-reviewer-opus) — feat/cast-agent-mvp

Reviewer: diff-reviewer-opus
Branch: feat/cast-agent-mvp @ 2fbba68 vs merge-base 8ca9cdd
Diff: .cue/cast-agent-mvp/tmp/1785214540-2fbba68/branch.diff

## Verdict

No outright blocking correctness bug found. Concurrency model is sound and matches the reviewed design. One important issue (I1) worth addressing before relying on the recovery contract.

## Verified invariants (holding correctly)

- process_group(0) + kill_on_drop(true) — supervisor.rs:175-176
- Parent env inherited, no env_clear() (C5) — supervisor.rs:177-178
- Prompt written in its own task + drop(stdin) for EOF (B2c) — supervisor.rs:197-202
- Separate dedicated stdout + stderr reader tasks — supervisor.rs:207-229
- Control-plane select! holds only cheap futures — supervisor.rs:236-252
- PGID captured once as i32 post-spawn — supervisor.rs:184
- Split guard: two-phase teardown + SIGKILL-only non-blocking Drop — supervisor.rs:86-98, 292-309
- Double-signal escalation via select! on signal streams during grace — supervisor.rs:302-307
- B2a re-wait() after non-wait arm — supervisor.rs:258-266
- C1 stdout reader join bounded with timeout + disk fallback — supervisor.rs:270-273
- Sync std::fs::BufWriter on dedicated blocking thread via std::sync::mpsc, flush per line — supervisor.rs:121-143
- Persist-before-spawn ordering — run.rs:65-74
- Atomic result.json (.tmp + rename) — finalize.rs:312-318
- Exit-code table 0/1/2/3/4/5 — finalize.rs:203-211 + main.rs:65

## Blocking issues

None.

## Important issues

### I1. C1 only half-implemented: the stderr reader join can hang past its "timeout"
supervisor.rs:274 bounds the stderr join, but supervisor.rs:279-283 then does an UNBOUNDED writer-thread join:

```rust
let _ = tokio::task::spawn_blocking(move || {
    let _ = stdout_writer.join();
    let _ = stderr_writer.join();
}).await;
```

The writer threads only exit once their Sender is dropped. stdout_tx/stderr_tx senders are owned by the reader TASKS. If a reader task was abandoned by the timeout at line 270/274 (the C1 grandchild-holds-the-pipe scenario), that task is NOT aborted (dropping a JoinHandle detaches, not cancels), so it still holds its Sender, so rx.recv() in the writer thread never returns Err, so writer.join() blocks FOREVER. The spawn_blocking wrapper does not help. Result: supervise() hangs, result.json never written — exactly the failure C1 was meant to prevent.

Impact: in the precise C1 scenario (grandchild inherits the pipe write-end and never closes it), cast-agent hangs after the child is reaped.

Fix options: bound the blocking join with its own timeout and detach on expiry; or stdout_task.abort()/stderr_task.abort() before joining the writer threads so the senders drop; or have the writer thread also select on a shutdown signal.

### I2. Prompt-writer task errors fully swallowed
supervisor.rs:197-202 spawns the stdin writer and discards the result of write_all (let _ = ...). An EPIPE on the prompt write (child died instantly before reading stdin) is silently dropped. A harness that needs the prompt yields Outcome::Completed with final_message: None and zero events — indistinguishable from a legitimate empty completion. Recommend at minimum logging the write error to stderr.log or tracing.

### I3. Interrupt during the reap/join tail window is not observed
After the control-plane select! returns (supervisor.rs:253), guard.kill_now(), the reap child.wait().await (line 263), and the two bounded reader joins (lines 270-274) run with NO signal handling. A SIGINT/SIGTERM arriving here is buffered but never acted upon; operator's second Ctrl-C does nothing until the joins complete or time out. Impact minor (windows short, child already being killed), but a real gap vs the "cast-agent is always responsive to signals" story.

### I4. Missing test coverage for two stated load-bearing invariants
- C1 grandchild-holds-the-pipe / disk fallback: no test drives a child that spawns a grandchild inheriting stdout and then hangs, forcing the reader-join timeout and the events_from_disk fallback. This is the exact path that masks I1. events_from_disk is entirely untested.
- Reader-task death observability: panics were made unreachable by construction (file creation moved out of the task into spawn_line_writer with ? at supervisor.rs:206, 222). Legitimate resolution, but a future edit reintroducing a panic would surface only as a JoinError at line 270, silently falling back to disk. Worth a comment asserting "task body must remain panic-free" and/or a test.

### I5. Outcome::Failed extracts no final message even when child produced usable text
run.rs:79-83 gates extract_result on outcome == Outcome::Completed. A non-zero exit that still emitted a terminal text event will have final_message: null while the text sits in stream.jsonl. Design defers best-effort extraction on non-happy paths, so consistent — confirm the orchestrator contract intends final_message strictly happy-path.

## Nits / suggestions

- N1 — Dead/misleading code: EndReason::child_signal (supervisor.rs:311-317). Never called. Remove.
- N2 — spawn_line_writer swallows write errors mid-stream (supervisor.rs:132-137): on write error it breaks silently, truncating stream.jsonl with no diagnostic. One-line eprintln!/tracing warn would aid post-mortem.
- N3 — duration_ms: u128 (finalize.rs:233). serde_json serializes fine but some strict consumers reject > 2^53. Cosmetic; consider u64.
- N4 — text_of_event tolerant extractor (opencode.rs:470-477): does not handle part being an ARRAY of content blocks. Fine for opencode MVP (single object); noting it's opencode-object-shaped, not fully harness-agnostic.
- N5 — extract_result picks the last text event unconditionally (opencode.rs:495-504). If opencode ever interleaves a text event after the final answer, returns wrong message. Acceptable; assumption worth a re-verification note.
- N6 — Test workaround live_group_members via ps (supervisor_test.rs:1767-1794, interrupt_test.rs:1258-1277): SOUND. Filtering to non-Z states asserts the real invariant ("no live process escaped the kill"). Does NOT mask a real bug — a genuinely-escaped live process would show a non-Z stat and fail. nativeCheckInputs = [procps] in flake.nix:1970 correctly pins ps.
- N7 — PATH-shim mechanism (interrupt_test.rs:1319-1353): clear security improvement over CAST_AGENT_FAKE_CMD. Exercises the real base_command() -> PATH-lookup -> exec path. $$-not-in-a-subshell caveat correctly documented.
- N8 — no_trailing_newline_still_parses: writer thread appends \n to every line (supervisor.rs:135), so stream.jsonl is always newline-terminated. Worth a one-line comment since it's a subtle normalization the recovery reader depends on.

## Summary

The MVP is solid and faithfully implements the reviewed design. The one issue to address before relying on the recovery contract is I1 (the unbounded writer-thread join defeating C1's bounded reader join) — it reintroduces exactly the hang C1 was designed to prevent, and is currently untested (I4). Everything else is robustness polish or acceptable-for-MVP deferrals. No security concerns; the PATH-shim refactor and env-inheritance decisions are correct.
