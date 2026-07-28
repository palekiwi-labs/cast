---
status: complete
priority: high
refs:
- .cue/cast-agent-mvp/spec/index.md
- .cue/cast-agent-mvp/plan/index.md
- .cue/cast-agent-mvp/plan/1785127643-8ca9cdd/phase-1-mvp.md
- .cue/cast-agent-mvp/note/recovery-outcome-contract.md
- .cue/cast-agent-mvp/doc/streaming-supervisor-design.md
- .cue/cast-agent-mvp/doc/cast-agent-architecture.md
- .cue/cast-agent-mvp/doc/opencode-headless-run-contract.md
- .cue/master/task/cast-agent-mvp.md
---
# TODO: Review the cast-agent MVP plans (reviewer prep)

Hand this to the consultant/reviewer before they critique the expanded MVP
scope. It lists what to read, in what order, the concepts to hold in mind, and
the specific questions we want answered.

## What we're asking the reviewer to do

Critique the **expanded MVP scope + the streaming-supervisor/recovery design**
for `cast-agent`, and tell us whether the scope line is right and whether the
technical design is sound before we settle the final version and start
building.

## Read in this order

1. **`spec/index.md`** (the WHY / intent + scope). Start here. Note the
   top-of-file REVIEW NOTE and the inline SCOPE-CHANGE FLAG in "Scope
   boundary" — that flag is the central question.
2. **`note/recovery-outcome-contract.md`** (the discussion record / richest
   rationale). The three "Resolved decisions (session N)" blocks + the
   "Ctrl-C / interrupt semantics" section carry the reasoning behind the
   scope change.
3. **`doc/streaming-supervisor-design.md`** (the concurrency design). The
   reader-task architecture, the annotated Rust sketch, and the 7-case
   failure-mode test matrix.
4. **`plan/index.md`** (the HOW roadmap). "Streaming supervisor + recovery
   contract (design summary)", the run-dir layout, exit-code table, revised
   "Key design decisions", and "Open questions".
5. **`plan/1785127643-8ca9cdd/phase-1-mvp.md`** (the executive plan). Slice 1c
   is where the design becomes concrete steps + tests.
6. **`.cue/master/task/cast-agent-mvp.md`** (the board card / acceptance
   criteria). AC 6 (interrupt) + AC 7 (artifacts/result.json) are the new ones.

Supporting background (read only if they want to go deeper):
- **`doc/cast-agent-architecture.md`** — the three orthogonal axes; the
  `Harness` trait vs orchestrator split; the full priority roadmap.
- **`doc/opencode-headless-run-contract.md`** — the opencode JSON-lines dialect
  (first adapter); note the `error` event that motivates the deferred in-stream
  detector.

## Source-code touchpoints the reviewer should know

- `crates/cast/src/mcp/exec.rs` — the EXISTING process-group supervisor we
  port from: `ProcessGroupGuard` (37-47), `kill_process_group` (52-65),
  `process_group(0)` (84), `kill_on_drop(true)` (87), timeout `select!`
  (118-141), and `test_timeout_kills_entire_process_tree` (421-488). KEY
  divergence: exec.rs buffers via `wait_with_output()`; we must stream.
- `crates/cast/src/dev/agent.rs` + `dev/claudecode/mod.rs:15-32` — the
  name()/base_command() identity-vs-executable precedent our `Harness` trait
  mirrors.
- `crates/cast-mcp-client/Cargo.toml` + `flake.nix:48-73` — the crate-manifest
  and per-crate Nix `buildRustPackage` templates for the new crate.

## Concepts the reviewer must hold in mind

- **Two-layer split**: Layer 1 = the `cast-agent` CLI mechanism (this work);
  Layer 2 = MCP wrapper tools in `cue-plugins` (later). We're only reviewing
  Layer 1.
- **Harness trait vs orchestrator**: the trait encapsulates ONLY what differs
  per harness (identity, headless args, result extraction); everything else
  (prompt input, run-dir, supervision, signals) is harness-agnostic.
- **Receipt vs evidence**: `result.json` (small structured verdict) vs
  `stream.jsonl` (raw firehose trace). The stream can't say WHY a run stopped;
  the supervisor can.
- **Live-flush is dual-purpose**: `tail -f` observability AND crash-durability
  (trace survives even a SIGKILL of cast-agent). This is why buffered output
  was rejected.
- **process_group(0) double duty**: (a) killpg the whole subtree; (b) insulate
  the child from terminal Ctrl-C so cast-agent is the sole signal recipient and
  orchestrates teardown / writes result.json.
- **Control-plane select! vs reader tasks**: readers are independent tasks;
  the parent select! only holds cheap control futures (wait/timeout/signals),
  so the timeout fires even against a silent-but-alive child.
- **Passive vs active supervision**: MVP is passive (exit/timeout/signals);
  the "fancy detectors" (silence watchdog, in-stream error sensing) are
  deferred but seam-compatible.
- **Continuation is caller-side** (trace-as-context); NO native harness resume.

## Specific questions we want answered

1. **Scope**: is the outcome/artifacts/signal contract rightly in the MVP, or
   should any of {run-dir, result.json, signal handling} be deferred? Our bet:
   the recovery loop is the core value and the seams are cheap now / expensive
   to retrofit.
2. **Concurrency design**: is the reader-task + control-plane-`select!` model
   correct and free of deadlock/starvation? Any cancellation-safety or
   pipe-buffer pitfalls we missed?
3. **Signal semantics**: SIGTERM -> 3s grace -> SIGKILL, double-signal
   escalation, guard-vs-kill_now double-kill avoidance — sound? Grace value?
4. **result.json schema**: are the fields reliably populatable across
   opencode/claudecode/pi? Anything missing/over-specified?
5. **Exit-code table**: explicit table (0/1/2/3/4/5) vs shell 128+signal — ok?
6. **Run-dir**: `${TMPDIR:-/tmp}/cast-agent/runs/` default + precedence +
   retention (keep-everything, no GC) — acceptable?
7. **Test matrix**: does the 7-case matrix + AC6/AC7 adequately prove
   correctness under Nix sandbox constraints?

## Open items still unsettled (flag if the reviewer has opinions)

- Final `result.json` field list + per-harness populatability (opencode first).
- child stderr: tee to BOTH `stderr.log` and cast-agent's stderr, or file only.
- run-dir retention/cleanup policy.
- claudecode `--verbose` requirement + pi JSON event shape (unverified).
