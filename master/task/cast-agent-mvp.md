---
title: cast-agent MVP
status: closed
priority: normal
refs:
- /home/pl/code/palekiwi-labs/cast/.cue/cast-agent-mvp/spec/index.md
- .cue/cast-agent-mvp/plan/index.md
- .cue/cast-agent-mvp/doc/cast-agent-architecture.md
---
# cast-agent MVP

Introduce a new `cast-agent` binary crate in the cast workspace that launches
a supported agent harness (`opencode`, `claudecode`, `pi`) as a **separate,
supervised child process** in **headless mode with JSON-lines streaming**, and
returns the final assistant message as plain text.

This is the process-isolated alternative to harnesses' in-process
subagent/task tools: the caller can monitor the stream, enforce a timeout, and
kill the whole process group cleanly.

The design was settled in the `cast-agent-design` context in the
`palekiwi/palekiwi` workspace. The authoritative architecture and the phased
roadmap have been injected into this task's context:

- Design reference: `.cue/cast-agent-mvp/doc/cast-agent-architecture.md`
- opencode headless contract: `.cue/cast-agent-mvp/doc/opencode-headless-run-contract.md`
- Spec (intent + scope): `.cue/cast-agent-mvp/spec/index.md`
- Phased plan: `.cue/cast-agent-mvp/plan/index.md`

## Scope (Priority 1 — MVP only)

`cast-agent run --harness <opencode|claudecode|pi> [--file <path> | stdin |
positional]`:

- headless + foreground + supervised; runs in cast-agent's own cwd
- spawns the harness binary in its own process group
- consumes the JSON-lines stream, **live-flushing** the raw trace to a run-log
- extracts and returns the final assistant text as plain text (stdout) on the
  happy path
- supervises with a wall-clock timeout and clean process-group SIGKILL
- **produces a per-run artifact bundle** (run directory: `prompt.txt`,
  `stream.jsonl`, `stderr.log`, `cast-agent.pid`, `result.json`) so the caller
  gets meaningful feedback + a recoverable trace on every exit path
- **`result.json`** = structured harness-agnostic verdict (outcome,
  final_message, exit, pointers, metadata); distinct process exit code per
  outcome
- **reacts to `SIGINT`/`SIGTERM`** with a graceful process-group teardown
  (SIGTERM -> 3s grace -> SIGKILL) -> `result.json(outcome=interrupted)`;
  serves interactive Ctrl-C and programmatic PID-targeted stops

> SCOPE EXPANDED during design (2026-07-27): the outcome/artifacts/signal
> contract was pulled into the MVP because the recovery loop is the core value
> over a plain wrapper. Rationale + full trail: `spec/index.md` (scope-change
> flag), `note/recovery-outcome-contract.md`, `doc/streaming-supervisor-design.md`.

Out of scope for this task (later priorities): git worktrees, the flag
vocabulary beyond `--harness`/`--file`/`--run-dir`, native session/resume
(MVP continuation is caller-side trace-as-context), the "fancy detectors"
(silence watchdog + in-stream error detection — passive supervision only for
now), the multiplexer, and the Layer-2 MCP wrapper tools (those live in
`cue-plugins`).

## Acceptance Criteria

1. **New `cast-agent` binary crate exists and builds.**
   - Verify by: `cargo build -p cast-agent`
   - Evidence: `cargo build -p cast-agent` exit 0; crate in workspace
     `members`, clap `run` subcommand renders `--help` (commit `c650dfc`).

2. **`Harness` trait + opencode/claudecode/pi adapters implemented.**
   - `Harness` carries `name` / `base_command` / `headless_args` /
     `extract_result`; flag-mapper methods (`model_args`, `escalate_args`,
     `system_prompt_args`, `agent_args`) exist as `None`-returning stubs.
   - Verify by: unit tests for each adapter's `headless_args` and
     `extract_result` against captured JSON-lines fixtures.
   - Evidence: `cargo test -p cast-agent` exit 0; `OpenCode` adapter unit
     tests (headless_args, last-text, multi-text, no-text) green
     (`tests/opencode_test.rs`, commits `dddc8b7` + `f0c4177`). claudecode/pi
     DEFERRED to fast-follow per scope decision (opencode-only MVP) — their
     JSON shapes/flags remain UNVERIFIED.

3. **`cast-agent run` drives a real harness end-to-end (headless).**
   - Spawns the child in its own process group, pipes the prompt via stdin,
     consumes the JSON-lines stream, returns the final assistant text.
   - Verify by: unit/integration tests for stream consumption + extraction;
     manual smoke run against `opencode run --format json` inside the cast
     devshell.
    - Evidence: AUTOMATED PORTION MET — `cargo test -p cast-agent` exit 0;
      3 e2e `orchestrate` tests (completed/failed/timed_out) drive a scripted
      `sh` fake harness through the full spawn-stream-extract-result.json path
      (`tests/orchestrate_test.rs`, commit `d39b279`). MANUAL SMOKE
      HUMAN-ATTESTED 2026-07-28 — real `opencode run --format json` run;
      live `tail -f stream.jsonl` streaming + Ctrl-C interrupt confirmed;
      covers the `part.text` vs `text` field verification.

4. **Process supervision: wall-clock timeout kills the whole process group.**
   - Mirrors the pattern in `crates/cast/src/mcp/exec.rs:37-65`
     (`ProcessGroupGuard` + `kill(-pgid, SIGKILL)`).
   - Verify by: a timeout test that asserts the child + its grandchildren are
     dead after the deadline (see the existing
     `test_timeout_kills_entire_process_tree`).
   - Evidence: `cargo test -p cast-agent` exit 0;
     `timeout_kills_entire_process_group` green
     (`tests/supervisor_test.rs`, commit `fe555b4`). Asserts no LIVE group
     members via a `ps -eo pid,pgid,stat,comm` poll rather than `killpg` —
     this container's pid 1 does not reap zombies, so `killpg` cannot prove
     death; supervisor correctly SIGKILLs the whole group (diagnosis in log
     entry `fe555b4`).

5. **Tests pass under Nix reproducible-build constraints.**
   - No reliance on `$HOME`/network; side effects routed to temp dirs
     (`std::env::temp_dir()`), per cast AGENTS.md testing guidelines.
   - Verify by: `nix build` (or the project's test entrypoint)
   - Evidence: `nix build .#cast-agent` reaches installPhase with the full
     test suite passing in the reproducible sandbox; `nativeCheckInputs` =
     bash/coreutils/procps (commit `3ada0d3`).

6. **Signal interruption yields a clean, recoverable result.**
   - `SIGINT`/`SIGTERM` to cast-agent triggers a graceful process-group
     teardown (SIGTERM -> 3s grace -> SIGKILL) and writes
     `result.json(outcome=interrupted)` with the partial trace intact.
   - Verify by: an interrupt test (spawn cast-agent, signal it mid-run) that
     asserts the outcome + a dead process group (`killpg(pgid,0) == ESRCH`);
     manual Ctrl-C smoke run.
   - Evidence: AUTOMATED PORTION MET — `cargo test -p cast-agent` exit 0; 3
     interrupt tests in `tests/interrupt_test.rs` (SIGINT -> interrupted,
     double-signal escalation during grace, trap-SIGTERM -> SIGKILL after 3s
     grace) using a `#!/bin/sh` `opencode` shim on PATH (PATH-substitution,
     exercising the real `base_command()` -> PATH spawn path); group death
      asserted via `live_group_members` poll (same non-reaping-pid-1 rationale
      as AC 4) (originally commit `00c6ca1`; PATH-shim refactor `2fbba68`).
      MANUAL CTRL-C SMOKE HUMAN-ATTESTED 2026-07-28 — confirmed
      `result.json(outcome=interrupted)` + no orphaned processes.

7. **Per-run artifact bundle + structured `result.json` are produced.**
   - Every run creates a run directory (`prompt.txt`, `stream.jsonl` live-
     flushed, `stderr.log`, `cast-agent.pid`, `result.json`); `result.json`
     carries the verdict fields (outcome, final_message, exit, log_path,
     prompt_path, metadata) on completed and failed outcomes.
   - Verify by: unit/integration tests asserting the bundle contents and the
     `result.json` shape for `completed` and `failed`.
   - Evidence: `cargo test -p cast-agent` exit 0; 7 `finalize` unit tests
     drive `classify` + `write_result_json` (atomic tmp+rename) via
     `ExitStatus::from_raw` for code + signal paths (`src/finalize.rs`,
     commit `eeb4c18`); 3 e2e `orchestrate` tests assert the bundle is
     produced on completed/failed/timed_out outcomes
     (`tests/orchestrate_test.rs`, commit `d39b279`).
