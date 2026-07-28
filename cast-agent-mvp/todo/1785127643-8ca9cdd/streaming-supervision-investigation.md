---
status: complete
priority: high
refs:
- .cue/cast-agent-mvp/plan/1785127643-8ca9cdd/phase-1-mvp.md
- .cue/cast-agent-mvp/plan/index.md
- .cue/cast-agent-mvp/doc/cast-agent-architecture.md
---
# Investigate: streaming supervision (live run-log) for the MVP

## Decision (settled)

Live streaming IS part of the MVP. The run-log MUST be written
**incrementally** as JSON-lines arrive, so an operator can `tail -f` it to
observe a running delegation in real time. The buffered `wait_with_output()`
approach (write-log-all-at-once-on-exit) is **rejected** — it defeats
observability, which is a core reason cast-agent exists.

## Why this needs investigation before implementing Slice 1c

The `cast` crate's existing supervisor at `crates/cast/src/mcp/exec.rs`
buffers everything via `child.wait_with_output()` and only touches output
after the process exits. cast-agent cannot reuse that path verbatim; it must
consume stdout **line-by-line while the child is still running** AND keep the
exact same process-group timeout-kill guarantees. Reconciling live streaming
with the supervision/timeout machinery is the non-trivial part.

## Problem statement

Design a supervisor loop that simultaneously:

1. Reads the child's stdout as it is produced (line-by-line JSON-lines),
   parses each line to `serde_json::Value`, and appends the RAW line to the
   run-log file immediately (flushed, so `tail -f` sees it live).
2. Collects the parsed events for `extract_result` at the end.
3. Enforces a wall-clock `--timeout`, and on the deadline SIGKILLs the entire
   process group (child + grandchildren) via the ported `ProcessGroupGuard` /
   `kill_process_group` from `exec.rs`.
4. Preserves whatever partial output was already streamed/logged when a kill
   occurs (no loss of the live log up to the kill point).

## Questions to answer

- **Loop shape**: single `tokio::select!` with three arms
  (line-read future, child-wait future, timeout sleep)? Or a spawned reader
  task draining stdout into a channel + a supervise/select on the parent?
  Which composes better with the RAII group-kill guard on every exit path
  (normal end, timeout, read error, future-drop)?
- **stderr**: the child also emits stderr (diagnostics). Stream it
  concurrently with stdout without deadlocking on full pipe buffers. Two
  reader tasks? `tokio::io::BufReader::lines()` on each?
- **Backpressure / pipe-buffer deadlock**: confirm we never block the child
  by failing to drain a pipe (the classic reason `wait_with_output` exists).
- **Run-log addressing**: path under a temp dir (Nix-safe, no `$HOME`),
  reported on stderr so a human can tail it. Naming / uniqueness scheme.
- **Flush semantics**: line-buffered vs explicit flush per line so `tail -f`
  is genuinely live.
- **Timeout on a stalled reader**: if the child is alive but silent, the
  read future pends forever — the timeout arm must still fire and kill. Verify
  select! biasing does not starve the timeout.
- **Result on timeout**: return an error (timeout), but the streamed run-log
  and any collected events up to the kill are retained. Confirm contract.

## Reference material

- `crates/cast/src/mcp/exec.rs` — port `ProcessGroupGuard` (37-47),
  `kill_process_group` (52-65), `process_group(0)` (84),
  `kill_on_drop(true)` (87); model AC-4 test on
  `test_timeout_kills_entire_process_tree` (421-488). Do NOT reuse its
  buffered `wait_with_output` output path.
- `doc/opencode-headless-run-contract.md` — JSON-lines shape; run loop ends on
  `session.status == idle` or process exit.

## Acceptance for the investigation

Produce a chosen loop design (with the concurrency primitives named) plus a
test strategy that proves: (a) lines are logged live/incrementally, (b) the
timeout kills the whole process group, (c) partial log survives a kill,
(d) no pipe-buffer deadlock. Then update the Slice 1c steps in
`plan/1785127643-8ca9cdd/phase-1-mvp.md` accordingly.
