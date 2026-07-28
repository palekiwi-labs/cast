---
status: open
refs:
- .cue/master/task/cast-agent-mvp.md
- .cue/cast-agent-mvp/spec/index.md
- .cue/cast-agent-mvp/doc/cast-agent-architecture.md
- .cue/cast-agent-mvp/doc/opencode-headless-run-contract.md
- .cue/cast-agent-mvp/doc/streaming-supervisor-design.md
- .cue/cast-agent-mvp/note/recovery-outcome-contract.md
---
# cast-agent — Master Plan

> Task context: `cast-agent-mvp`. See `spec/index.md` for intent/scope and
> `doc/cast-agent-architecture.md` for the authoritative architecture. This
> plan holds the phased roadmap and the concrete MVP implementation approach,
> grounded in the cast workspace's actual structure.

## Problem

See `spec/index.md`. In short: replace in-process, hard-to-govern harness
subagents with a **process-isolated, supervised** headless launcher that
returns the final assistant message as plain text.

## Where it lives in the cast workspace

- New **third crate**: `crates/cast-agent` (binary), added to the workspace
  `members` in the root `Cargo.toml` alongside `cast` and `cast-mcp-client`.
- Model the crate manifest on `crates/cast-mcp-client/Cargo.toml`
  (edition 2024; `anyhow`, `clap` derive, `serde`/`serde_json`, `tokio`
  multi-thread + macros + `process`/`signal`, `tracing`/`tracing-subscriber`;
  dev-deps `assert_cmd`, `predicates`, `tempfile`, `tokio-util`).
- Reuse patterns already proven in the `cast` crate:
  - **Process-group supervision**: `crates/cast/src/mcp/exec.rs:37-65`
    (`ProcessGroupGuard` + `kill(-pgid, SIGKILL)`, `cmd.process_group(0)`,
    `kill_on_drop(true)`, `tokio::select!` timeout arm). Port/adapt this into
    cast-agent's supervisor rather than reinventing it.
  - **Harness identity vs executable split**: `crates/cast/src/dev/agent.rs:18`
    and `crates/cast/src/dev/claudecode/mod.rs:15-32`
    (`name()="claudecode"`, `base_command()="claude"`). cast-agent's `Harness`
    trait is an INDEPENDENT trait (different concern: headless invocation, not
    container mounting) but mirrors the naming convention.

## Architecture — two layers (recap)

- **Layer 1 — `cast-agent` CLI (this plan): the mechanism.** Single `run`
  command + `--harness`; JSON-lines stream consumption; harness-aware
  final-message extraction; a per-run **artifact bundle** (run directory) with
  a live-flushed trace and a structured `result.json` verdict; process-group
  supervision with **wall-clock timeout AND signal-driven graceful teardown**;
  plain-text final-message output on the happy path.
- **Layer 2 — MCP wrapper tools: ergonomics (LATER, NOT here).** Lives in
  `cue-plugins`, requires a deployed cast-agent MVP first. Out of scope.

## Streaming supervisor + recovery contract (design summary)

Two design docs carry the detail; this is the orientation for a reviewer:

- **Concurrency model** (`doc/streaming-supervisor-design.md`): dedicated
  **reader-task (actor) pattern**, NOT a mega `tokio::select!`. The child is
  spawned in its own process group with stdin/stdout/stderr piped; the prompt
  is written to stdin then EOF'd. One task drains stdout (parse each JSON line,
  append raw + flush to the run-log for a live `tail -f`, collect events); one
  task drains stderr (prevents pipe-buffer deadlock). The parent's `select!`
  is a pure **control plane** over cheap futures — `child.wait()`, the
  wall-clock timeout, and the signal streams — so the timeout fires even when
  the child is alive but silent. A refined `ProcessGroupGuard` (with
  `kill_now()` that `take()`s the pgid to avoid double-kill) backstops every
  exit path including future-drop. Only the kill/guard machinery is ported from
  `exec.rs`; its buffered `wait_with_output()` output path is explicitly NOT
  reused (it would defeat live observability and crash-durability).
- **Outcome + artifacts** (`note/recovery-outcome-contract.md`): every run
  yields a run directory (`prompt.txt`, `stream.jsonl`, `stderr.log`,
  `cast-agent.pid`, `result.json`); `result.json` is the harness-agnostic
  verdict (outcome, final_message, exit, pointers, metadata) referencing the
  trace; distinct exit code per outcome; SIGINT/SIGTERM -> SIGTERM child group
  -> 3s grace -> SIGKILL -> write `result.json(outcome=interrupted)`.

### Run directory: location + layout

- Base-dir precedence: `--run-dir <path>` flag > `CAST_AGENT_RUN_DIR` env >
  default `${TMPDIR:-/tmp}/cast-agent/runs/`. The default is Nix-safe and
  **unifies test and prod paths** (tests use `std::env::temp_dir()` == TMPDIR),
  exercising the same code and avoiding `$HOME`.
- Per-run subdir is timestamp-first (sortable, mirrors cue's
  `<timestamp>-<hash>`), e.g. `20260727T153000Z-opencode-<pid>/`.
- The run-dir path is printed to **stderr at startup** (before child output) so
  an operator can tail immediately, and echoed in `result.json`. The run-dir is
  the single addressable **handle** for the run — which also sets up Phase 4
  (the multiplexer, where a detached run *returns* such a handle).

### Exit-code table (explicit; adjustable)

`0` completed | `1` failed (child non-zero exit) | `2` usage error |
`3` timed_out | `4` interrupted (signal) | `5` crashed (child killed by signal
/ spawn failure).

## Phase 1 — MVP (current focus)

`cast-agent run --harness <opencode|claudecode|pi> [--file <path> | stdin |
positional]`.

### 1a. Crate scaffold
- [ ] Add `crates/cast-agent` to the workspace `members`.
- [ ] `Cargo.toml` modeled on `cast-mcp-client`; `main.rs` with a clap
      `run` subcommand (`--harness` required enum; `--file` optional;
      positional prompt; `--timeout` optional with a sane default;
      `--run-dir` optional override for the run directory).
- [ ] `cargo build -p cast-agent` succeeds (AC 1).

### 1b. `Harness` trait + adapters (TDD)
- [ ] Define the `Harness` trait: `name`, `base_command`, `headless_args`,
      `extract_result(&[serde_json::Value]) -> Option<String>`; flag-mapper
      methods (`model_args`, `escalate_args`, `system_prompt_args`,
      `agent_args`) as `None`-returning default stubs.
- [ ] `opencode` adapter first: `headless_args() = ["run", "--format",
      "json"]`; `extract_result` returns the last `text` event's text
      (see `doc/opencode-headless-run-contract.md`).
- [ ] `claudecode` adapter: `name()="claudecode"`, `base_command()="claude"`,
      `headless_args()=["-p", "--output-format", "stream-json"]` (+ any
      required verbose/streaming flags — verify against the claude headless
      reference in the design context); extractor for its `result` event.
- [ ] `pi` adapter: `headless_args()=["--mode", "json"]`; extractor for its
      final assistant event.
- [ ] Unit tests: for each adapter, assert `headless_args` and drive
      `extract_result` against **captured JSON-lines fixtures** (AC 2).

### 1c. Orchestrator: prompt input + spawn + stream + supervise (TDD)

> Design resolved: `doc/streaming-supervisor-design.md` (reader-task pattern)
> and `note/recovery-outcome-contract.md` (outcome + artifacts + signals). The
> concrete step list lives in the executive plan `phase-1-mvp.md`; this is the
> phase-level summary.

- [ ] Prompt resolution (harness-agnostic): `--file` > stdin > positional; pipe
      it to the child's stdin uniformly AND persist a copy to
      `<run-dir>/prompt.txt` (so continuation works even for stdin/positional).
- [ ] Establish the run directory (precedence: `--run-dir` > env > TMPDIR
      default); write `cast-agent.pid`; print the run-dir path to stderr.
- [ ] Spawn the child in its own process group (`process_group(0)`,
      `kill_on_drop(true)`), stdin/stdout/stderr piped. Dedicated reader tasks:
      stdout -> parse + append/flush each line to `stream.jsonl` + collect
      events; stderr -> drain to `stderr.log` (no pipe-buffer deadlock).
- [ ] Supervisor `select!` (control plane): `child.wait()`, wall-clock
      `--timeout`, `SIGINT`, `SIGTERM`. Any signal -> graceful teardown:
      SIGTERM the group -> 3s grace -> SIGKILL; timeout -> SIGKILL; ported
      `ProcessGroupGuard` backstops. Yields a `RunOutcome`
      (completed/failed/crashed/timed_out/interrupted).
- [ ] Finalize: call `extract_result` for the final message; write
      `result.json` (verdict + pointers) on EVERY outcome; print the final
      assistant text to **stdout** on the happy path; exit with the
      outcome-specific code.
- [ ] Supervision tests (AC 4 + interrupt AC): timeout kills child +
      grandchildren (modeled on `exec.rs`'s
      `test_timeout_kills_entire_process_tree`); an injected `SIGINT`/`SIGTERM`
      mid-run yields `result.json(outcome=interrupted)` and a dead process
      group. Full failure-mode matrix in `doc/streaming-supervisor-design.md`.
- [ ] End-to-end: integration test for stream consumption + extraction against
      a scripted fake harness; manual smoke run against real
      `opencode run --format json` in the cast devshell (AC 3), tailing
      `stream.jsonl` live and Ctrl-C'ing to exercise the interrupt path.

### 1d. Nix + hygiene
- [ ] Ensure the new crate builds under `nix build` and its tests obey the
      reproducible-build constraints (no `$HOME`/network; temp dirs only) — cast
      AGENTS.md testing guidelines (AC 5).
- [ ] Docs: add a `crates/cast-agent/docs/README.md` TOC entry per the cast
      docs convention (progressive-discovery READMEs).

## Later phases (out of scope for this task)

- **Phase 2 — git worktrees**: `--cwd` + `--worktree` (ephemeral vs named
  lifecycles). See architecture doc.
- **Phase 3 — practical flag enhancements**: `--model`, permission escalation,
  `--system-prompt`, `--agent`, filled in as `Option<Vec<String>>` trait
  mappers using the per-harness headless research docs (in the design context
  of `palekiwi/palekiwi`).
- **Phase 4 — multiplexer**: detached placement; changed return contract.
- **Separate track**: Layer-2 MCP wrapper tools in `cue-plugins`.

## Key design decisions

1. Build the **CLI first**; wrappers are a later `cue-plugins` phase.
2. **Dual output contract**: happy-path final message as plain text on stdout,
   AND a structured `result.json` verdict written on every outcome (revised
   from the earlier "plain-text only; structured deferred" — see the
   scope-change flag in `spec/index.md` and `note/recovery-outcome-contract.md`).
   The recovery loop is the core value proposition, so the outcome/artifact
   contract is MVP, not deferred.
3. **Signal handling is MVP.** SIGINT/SIGTERM -> graceful process-group
   teardown -> `result.json(outcome=interrupted)`. Serves interactive Ctrl-C
   and programmatic PID-targeted stops with one mechanism.
4. **Live-flushed trace is a hard requirement** (not buffered): it is both the
   `tail -f` observability channel and the crash-durability substrate that
   makes recovery possible. Rules out reusing `exec.rs`'s `wait_with_output`.
5. **Passive supervision only** for the MVP: react to process exit, wall-clock
   timeout, and signals. The "fancy detectors" (silence watchdog, in-stream
   error sensing) are deferred but the supervisor is structured to accept them
   as an extra `select!` control arm.
6. **Continuation is caller-side** (trace-as-context); no native harness resume.
7. **Permissions**: MVP relies on the harness's own resolved config; explicit
   escalation is a priority-3 flag.
8. **Parallelism** and the **multiplexer** are deferred.
9. Single `run` command + required `--harness` + `None`-returning flag-mapper
   stubs; no raw native-flag passthrough in the MVP.

## Open questions (deferred, tracked)

- Final `result.json` field list + which fields are reliably populatable per
  harness (opencode adapter first).
- Whether child stderr tees to BOTH `stderr.log` and cast-agent's own stderr,
  or file only.
- Session/resume id semantics differ per harness (native resume — later).
- Parallel spawn + process-tree observability across many concurrent runs.
- Silence watchdog + in-stream error detection (the deferred "fancy detectors").
- Run-dir retention/cleanup policy (currently: keep everything; no GC).
