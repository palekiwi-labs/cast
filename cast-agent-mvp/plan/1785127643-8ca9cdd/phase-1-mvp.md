---
status: open
refs:
- .cue/cast-agent-mvp/plan/index.md
- .cue/cast-agent-mvp/spec/index.md
- .cue/cast-agent-mvp/doc/cast-agent-architecture.md
- .cue/cast-agent-mvp/doc/opencode-headless-run-contract.md
- .cue/cast-agent-mvp/doc/streaming-supervisor-design.md
- .cue/cast-agent-mvp/note/recovery-outcome-contract.md
- .cue/master/task/cast-agent-mvp.md
---
# Executive Plan — cast-agent MVP (Phase 1)

## Foreword

This plan executes **Phase 1 (MVP)** of the `cast-agent-mvp` master plan
(`plan/index.md`). It creates a new `crates/cast-agent` binary crate that
launches a supported harness (`opencode` first, then `claudecode`, `pi`) as a
process-isolated, supervised child in headless JSON-lines mode. On the happy
path it returns the final assistant message as plain text on stdout; on **every**
path it writes a per-run artifact bundle (run directory) with a live-flushed
trace and a structured `result.json` verdict, and it reacts to `SIGINT`/`SIGTERM`
with a graceful process-group teardown.

Two companion docs are authoritative for Slice 1c and MUST be read before
implementing it: `doc/streaming-supervisor-design.md` (concurrency model +
failure-mode test matrix) and `note/recovery-outcome-contract.md` (outcome
taxonomy, run-dir layout, signal semantics, exit-code table).

## Pre-implementation findings (RESOLVED — bake these in)

The expanded design went through two adversarial Opus review passes (traces:
`trace/1785127643-8ca9cdd/opus-design-review-cast-agent-mvp.md` +
`opus-verification-of-design-review.md`). Both go/no-go unknowns are now
empirically closed; the remaining load-bearing findings are folded into Slice 1c
below as hard invariants.

Verified this session (empirical traces):

- **C6 (opencode self-exit) = GO** (`trace/.../opencode-self-exit-verification.md`).
  `opencode run --format json` self-exits `EXIT=0` ~50-200ms after the final
  JSON event on trivial AND tool-using runs. The `child.wait()`-based supervisor
  stands; no idle-detection fallback needed.
- **`drop(stdin)` is LOAD-BEARING** (same trace): opencode blocks reading stdin
  to EOF *before doing anything*. If stdin stays open the child never starts,
  `child.wait()` never resolves, and every run misclassifies as `timed_out`.
  Must be an explicit invariant AND a test (fake harness that reads stdin to EOF
  before emitting output must still supervise to clean Success).
- **`extract_result` "last `text` event" rule = CONFIRMED valid** (same trace):
  the terminal stream event is always `step_finish`, not a trailing `text`, so
  type-filtering to `text` and taking the last returns the answer. Carry-forward
  caveat only if a single message ever splits into multiple `text` parts.
- **Run-log writer decision = sync `std::fs::BufWriter` on a dedicated blocking
  thread, fed via a channel from the async stdout reader, flush PER LINE**
  (`trace/.../runlog-writer-flush-benchmark.md`). ~2.1M lines/sec, single
  `write(2)`/line, sub-ms `tail -f` latency, zero-loss on process death
  (partial-survival test PASSED). This REPLACES the `tokio::fs` sketch (24x
  slower; its `flush().await` is a near no-op). No tokio `fs` feature needed.
  Durability claim scoped precisely: survives cast-agent process death (SIGKILL),
  NOT host crash. Do NOT harden to `fsync`-per-line; do NOT use boundary/interval
  flush for the MVP.

Load-bearing must-fixes from the reviews (folded into Slice 1c):

- **C5 — inherit the parent env** for the harness child. Do NOT port
  `exec.rs`'s `env_clear()`: it drops `HOME`/API keys and makes 100% of real
  harness runs fail auth. The container is the sandbox boundary, so there is no
  isolation reason to clear.
- **C1 — bound BOTH reader-task joins with a timeout** (e.g. `timeout(2s, ...)`),
  falling back to the on-disk trace. A grandchild that inherits (or `setsid`s
  out with) the stdout/stderr pipe write-end means the reader never hits EOF and
  the join hangs forever. Applies to stdout AND stderr.
- **B2c — write the prompt to stdin in its own task** (not inline before
  spawning readers). A >64KiB prompt can deadlock; a fast-failing child EPIPEs
  `write_all().await?` and bails BEFORE any `result.json` is written.
- **B2a — re-`wait()`/`try_wait()` after any non-wait arm kills the group** to
  reap and classify code-vs-signal for `result.json.exit` (fills AC6/AC7).
- **Capture PGID once** as a plain `i32` immediately post-spawn; never re-read
  `child.id()` (returns `None` after reap — PID-reuse hazard).
- **Split the guard**: graceful two-phase (SIGTERM -> grace -> SIGKILL) on the
  interrupt/timeout arms; `Drop` is SIGKILL-only and NON-blocking (must not
  `sleep(grace)` on a runtime worker).
- **Double-signal escalation** must `select!` on the signal streams DURING the
  grace window (not a bare `sleep(grace)`); "any signal during grace escalates"
  (Tokio coalesces — do not count). Register both signal streams before the
  child work begins.
- **No latent panics / persist-before-spawn**: create+flush run-dir /
  `prompt.txt` / `cast-agent.pid` BEFORE spawn; a run-dir/stdin failure must
  still yield a `result.json` or a mapped exit code (no `.expect(...)` abort).
  Write `result.json` ATOMICALLY (tmp + rename) so a tailing orchestrator never
  sees a torn verdict.
- **Reader-death observability**: the control-plane `select!` must not be blind
  to a reader task panicking (else the trace silently truncates and the run
  drifts to timeout with no error).

Deferred / hygiene (noted, not MVP-blocking): opencode-only vs 3-adapter scope
is a PM call (claudecode/pi flags remain UNVERIFIED — keep them fast-follow);
event `Vec` is O(run) RAM (stream.jsonl holds the full record); schema
`version`/PID/argv niceties; tmpfs unbounded-growth + run-dir name collision
(add a sub-second/random suffix); **macOS unexamined** (killpg/SIGCHLD reaping
differ; ~16KiB pipe buffer makes the B2c deadlock likelier — verify on the
second platform).

Branch: `feat/cast-agent-mvp`. Follow TDD (red-green-refactor) throughout;
each adapter and the orchestrator get failing tests first. Commit granularly
per the `git-commit` skill; log to cue after each commit.

Grounding facts verified against source this session:

- Workspace `members` currently holds only `crates/cast` and
  `crates/cast-mcp-client` (`Cargo.toml:3-6`). Add the new crate here.
- Crate manifest model: `crates/cast-mcp-client/Cargo.toml` (edition 2024;
  `anyhow`, `clap` derive, `serde`/`serde_json`, `tokio` rt-multi-thread +
  macros; dev-deps `assert_cmd`, `predicates`, `tempfile`, `tokio-util`). We
  additionally need tokio features `process`, `signal`, `io-util`, `time`,
  `sync`, and `fs` (the last for `tokio::fs` run-log writes; `io-util` is NOT
  enabled anywhere in the workspace today and is required for
  `BufReader`/`AsyncBufReadExt`), plus `tracing` + `tracing-subscriber`.
- Process-group supervision to port/adapt: `crates/cast/src/mcp/exec.rs`
  — `ProcessGroupGuard` (37-47), `kill_process_group` (52-65),
  `cmd.process_group(0)` (84), `kill_on_drop(true)` (87), `tokio::select!`
  timeout arm (118-141), and the timeout test
  `test_timeout_kills_entire_process_tree` (421-488) as the model for AC 4.
  NOTE: exec.rs uses `wait_with_output()` (buffers all output). cast-agent
  needs to STREAM stdout line-by-line while supervising, so the select! loop
  differs — stream reader future vs timeout sleep.
- Harness identity/executable split precedent: `crates/cast/src/dev/agent.rs`
  (`Agent` trait, `name()`/`base_command()`) and
  `crates/cast/src/dev/claudecode/mod.rs:15-32` (`name()="claudecode"`,
  `base_command()="claude"`). cast-agent's `Harness` is an INDEPENDENT trait
  (headless invocation, not container mounting) but mirrors the naming.
- opencode contract (`doc/opencode-headless-run-contract.md`):
  `opencode run --format json`; JSON-lines, each object has `type`,
  `timestamp`, `sessionID`. Final result = the last `text` event's text.
  There is NO `-p`/`--print`; JSON is strictly `--format json`.
- Nix: `flake.nix:48-73` defines one `buildRustPackage` per crate with
  `cargoBuildFlags`/`cargoTestFlags = [ "-p" <crate> ]`. Add a `cast-agent`
  package mirroring the `cast-mcp-client` block. `nativeCheckInputs` (bash/jq)
  needed only if tests shell out to a scripted fake harness.
- Test constraints (AGENTS.md): Nix sandbox — no `$HOME`, no network; route
  all fs side effects (run-log, fixtures scratch) through `tempfile` /
  `std::env::temp_dir()`.

## Slice 1a — Crate scaffold

- [x] Add `"crates/cast-agent"` to `members` in root `Cargo.toml`.
- [x] Create `crates/cast-agent/Cargo.toml` modeled on `cast-mcp-client`
      (edition 2024; tokio rt-multi-thread+macros+process+signal+io-util+time+
      sync; dev-deps assert_cmd/predicates/tempfile/tokio-util; cfg(unix) libc).
- [x] Create `src/main.rs` with clap derive: `run` subcommand;
      `--harness` REQUIRED enum (**opencode-only** per scope decision — no
      default); `--file` optional; positional `prompt`; `--timeout` (default
      300s); `--run-dir` optional override. Body `todo!()` for now.
- [x] Update `Cargo.lock` (`cargo build -p cast-agent`).
- [x] Verify `cargo build -p cast-agent` succeeds (**AC 1**). Commit c650dfc.

## Slice 1b — `Harness` trait + adapters (TDD)

- [x] `src/harness/mod.rs`: define the `Harness` trait per architecture doc
      (72-101): `name`, `base_command`, `headless_args`,
      `extract_result(&[serde_json::Value]) -> Option<String>`; flag-mapper
      defaults (`model_args`, `escalate_args`, `system_prompt_args`,
      `agent_args`) returning `None`.
- [x] Add a `harness_from_kind(HarnessKind) -> Box<dyn Harness>` (or enum
      dispatch) bridging the clap enum to adapter instances. (Implemented as
      `HarnessKind::adapter()` in `main.rs:55-61`, wired into `run` at
      `d39b279`.)
- [x] RED: opencode unit tests — `headless_args() == ["run","--format",
      "json"]`; `extract_result` against a JSON-lines fixture (last `text`;
      multiple-text; no-text -> None).
- [x] GREEN: implement `OpenCode` adapter (`name/base_command="opencode"`).
      extract_result tolerant of `part.text` and top-level `text`.
- [~] DEFERRED (scope decision): `ClaudeCode` + `Pi` adapters. Their JSON
      shapes / required flags remain UNVERIFIED; MVP ships opencode only,
      claudecode + pi are fast-follow. (**AC 2** met by the opencode unit
      tests.)

## Slice 1c — Orchestrator: input + run-dir + spawn + stream + supervise + signals (TDD)

> DESIGN RESOLVED: follow `doc/streaming-supervisor-design.md` (dedicated
> reader-task / actor pattern, NOT a mega `tokio::select!`) and
> `note/recovery-outcome-contract.md` (outcome + artifacts + signals). Live
> incremental trace with per-line flush is a hard requirement.

### Prompt input + run directory

- [x] `src/prompt.rs`: harness-agnostic resolution `--file` > stdin >
      positional; return the payload as a String. Unit tests for precedence.
      (cef4626; persist-before-spawn done in run.rs / d39b279.)
- [x] `src/rundir.rs`: resolve the run-dir base (`--run-dir` > `CAST_AGENT_RUN_DIR`
      > `${TMPDIR:-/tmp}/cast-agent/runs/`); create a timestamp-first per-run
      subdir (`<ts>-<harness>-<pid>`; add a sub-second/random suffix to avoid
      recycled-PID collisions). PERSIST-BEFORE-SPAWN invariant: create+flush the
      run-dir, `cast-agent.pid`, and `prompt.txt` BEFORE spawning the child, so
      an instant crash/EPIPE cannot lose the artifacts the recovery contract
      promises. A run-dir creation failure must yield a mapped exit code, never
      a `.expect(...)` abort. Print the run-dir path to **stderr** at startup.
      Unit tests for base precedence + layout (temp-dir fs, Nix-safe).

### Supervisor (reader-task pattern)

- [x] `src/supervisor.rs`: port `ProcessGroupGuard` + `kill_process_group`
      from `exec.rs` (unix `cfg`), SPLIT into graceful two-phase (SIGTERM ->
      grace -> SIGKILL, used on the interrupt/timeout arms) vs a SIGKILL-only,
      NON-blocking `Drop` backstop; `kill_now(&mut self)` `take()`s the pgid so
      an explicit kill does not double-fire on drop. Capture the PGID ONCE as a
      plain `i32` immediately post-spawn (never re-read `child.id()`). Spawn
      `base_command` + `headless_args` with `process_group(0)`,
      `kill_on_drop(true)`, stdin/stdout/stderr piped, and the PARENT ENV
      INHERITED (do NOT `env_clear()` — C5).
- [x] Write the prompt to stdin in ITS OWN task (B2c — avoids >64KiB deadlock
      and EPIPE-before-result.json), then drop stdin (EOF). `drop(stdin)` is
      LOAD-BEARING: opencode blocks on stdin EOF before starting (verified).
- [x] Dedicated STDOUT reader task: `BufReader::lines()` -> per line, hand the
      raw line to the run-log writer AND parse to `serde_json::Value` (collect
      into a `Vec`). Run-log writer = sync `std::fs::BufWriter` on a dedicated
      blocking thread fed over a channel, `flush()` PER LINE (benchmark-backed;
      no tokio `fs` feature). Live `tail -f`; survives SIGKILL of cast-agent.
- [x] Dedicated STDERR reader task: drain to `<run-dir>/stderr.log` (prevents
      pipe-buffer deadlock).
- [x] Bound BOTH reader-task joins with a short `timeout` (C1) during teardown,
      falling back to the on-disk trace — a grandchild holding the pipe
      write-end means the reader never EOFs and the join would hang forever.
- [x] Control-plane `select!` (cheap futures only — NOT the readers):
      `child.wait()` | wall-clock `timeout` | `SIGINT` | `SIGTERM`
      (`tokio::signal::unix`). The timeout thus fires even when the child is
      alive but silent. Whichever arm fires runs the SAME teardown:
      - timeout: `kill_now()` (SIGKILL group);
      - signal: SIGTERM the group -> **3s fixed grace** -> `kill_now()` (SIGKILL);
        `select!` on the signal streams DURING the grace window so a second
        signal escalates immediately (any signal during grace escalates; do not
        count — Tokio coalesces). Register both signal streams before spawn.
      - normal exit: `kill_now()` is a harmless ESRCH.
      After the kill, re-`wait()`/`try_wait()` the child (B2a) to reap and
      classify code-vs-signal for `result.json.exit`. Then join both reader
      tasks (bounded per C1) and build a `RunOutcome`. The `select!` must also
      observe reader-task death (do not silently drift to timeout on a reader
      panic).
- [x] Outcome taxonomy (implemented as `supervisor::EndReason` +
      `finalize::Outcome` = Completed/Failed/Crashed/TimedOut/Interrupted rather
      than a single `RunOutcome` enum — cleaner split of raw end vs classified
      verdict). The RAII guard backstops the future-drop/cancel path.

### Finalize (artifacts + output + exit code)

- [x] Write `<run-dir>/result.json` on EVERY outcome, ATOMICALLY (write `.tmp`
      + rename — a tailing orchestrator must never see a torn verdict; the "log
      exists, no result.json = hard-killed" heuristic depends on this): `outcome`,
      `final_message` (when produced via `extract_result`), `exit{code,signal}`
      (keep the raw signal int alongside any name), `log_path`, `prompt_path`,
      `harness`, `event_count`, `duration_ms`. Scope `error_detail` (if present)
      to MVP-available sources only (stderr-tail / signal name); the passive MVP
      cannot produce in-stream error detail.
- [x] Happy path: print the final assistant text to **stdout**; diagnostics +
      run-dir path to **stderr**. Exit with the outcome-specific code
      (0 completed | 1 failed | 2 usage | 3 timed_out | 4 interrupted |
      5 crashed).
- [x] Add tokio features (`io-util`, `time`, `sync`) + `libc` (cfg-unix) to
      Cargo.toml (from Slice 1a). NOTE: `fs` NOT needed — the run-log writer is
      sync `std::fs::BufWriter` on a blocking thread (benchmark decision).

### Tests

- [x] RED: supervisor failure-mode matrix from
      `doc/streaming-supervisor-design.md` (scripted `sh`/`printf` fakes,
      temp-dir fs): (1) grandchild-leak / process-group kill = **AC 4**;
      (2) pipe-buffer deadlock; (3) silent-hang timeout; (4)
      live-flush-before-return; (5) partial-log survival; (6)
      no-trailing-newline; (7) future-drop cancellation. PLUS the review-added
      cases: (8) **grandchild holds stdout/stderr** (`sh -c 'sleep 100 | cat &'`)
      — assert supervise returns promptly after kill via the bounded join (C1),
      for BOTH pipes; (9) **stdin-EOF invariant** — fake harness reads stdin to
      EOF before emitting, must still reach clean Success (proves `drop(stdin)`);
      de-race the live-flush test (4) with a deterministic barrier (FIFO/byte
      the test controls), not a poll. (9 tests in
      `tests/supervisor_test.rs`, all green — `fe555b4`. The AC 4 group-kill
      assertion uses a no-live-members poll over `ps` rather than `killpg`
      because this container's pid 1 does not reap zombies — see log entry
      `fe555b4` for the diagnosis.)
- [x] RED: **interrupt** test (**AC 6**) — spawn cast-agent as a child, send it
      `SIGINT` (then a variant with `SIGTERM`) mid-run; assert
      `result.json.outcome == "interrupted"`, `stream.jsonl` holds the partial
      trace, and `killpg(pgid, 0) == ESRCH` (child group dead, no orphans).
      Automatable in the `exec.rs` test style. ADD: a **double-signal
      escalation** test and a **trap-SIGTERM** test (`trap '' TERM; sleep 100`
      must be SIGKILLed after the grace window). (3 tests in
      `tests/interrupt_test.rs` — SIGINT, double-signal escalation, trap-SIGTERM
      grace — all green via a `#!/bin/sh` `opencode` shim placed first on PATH
      (PATH-substitution, exercising the real `base_command()` -> PATH spawn
      path; originally `CAST_AGENT_FAKE_CMD` env override at `00c6ca1`, refactored
      to PATH-shim because the env var shipped a silent attacker-influencable
      substitution surface in release and made `result.json.harness` lie — see
      log entry `cast-agent-mvp`/`2fbba68`); `00c6ca1` + refactor `2fbba68`.
      Group-death asserted via `live_group_members` poll, same rationale as AC 4.)
- [x] RED: `result.json` shape test (**AC 7**) — drive a scripted harness to
      completion and to a non-zero exit; assert the verdict fields + pointers
      for `completed` and `failed`. (7 unit tests in `src/finalize.rs` drive
      `classify` + `write_result_json` via `ExitStatus::from_raw` for code and
      signal paths; `eeb4c18`. End-to-end bundle production on
      completed/failed/timed_out is covered by `tests/orchestrate_test.rs` at
      `d39b279`.)
- [x] RED: end-to-end stream test — a scripted fake harness that emits known
      JSON-lines; assert cast-agent prints the extracted final text (**AC 3**
      automated portion). GREEN: wire `main.rs run` to orchestrator.
      (3 e2e `orchestrate` tests inject a `sh` fake harness; `run.rs` +
      `main.rs` wired at `d39b279`. Full suite 35 tests green; exit 0.)
- [ ] Manual smoke: real `opencode run --format json` in the cast devshell;
      `tail -f <run-dir>/stream.jsonl` to confirm live streaming; Ctrl-C
      mid-run to confirm the interrupt path yields `result.json(interrupted)`
      and no orphaned processes. Capture as evidence for AC 3 + AC 6.
      (OUTSTANDING — needs a devshell with `opencode` installed; human
      attestation required.)

## Slice 1d — Nix + hygiene

- [x] Add a `cast-agent` `buildRustPackage` to `flake.nix` mirroring the
      `cast-mcp-client` block (`cargoBuildFlags`/`cargoTestFlags =
      ["-p","cast-agent"]`; add `nativeCheckInputs = [bash]` if tests use a
      shell fake harness). (`flake.nix:72-78`; `nativeCheckInputs` =
      bash/coreutils/procps for the `sh` fakes + `ps` group-death poll;
      `3ada0d3`.)
- [x] Run `nix build .#cast-agent` (or `nix flake check`) to confirm the crate
      builds and tests pass under the reproducible sandbox (**AC 5**).
      (`nix build .#cast-agent` reaches installPhase with the full test suite
      passing in the sandbox — log entry `3ada0d3`.)
- [x] Create `crates/cast-agent/docs/README.md` as a progressive-discovery TOC
      per the cast docs convention. (`3ada0d3`.)

## Acceptance criteria mapping

> NOTE: the task card's AC list needs to grow to match the expanded scope
> (interrupt + artifacts/result.json). Proposed additions flagged below; task
> card `.cue/master/task/cast-agent-mvp.md` updated in lockstep.

- AC 1 (builds): Slice 1a — `cargo build -p cast-agent`.
- AC 2 (adapters + extraction): Slice 1b unit tests vs fixtures.
- AC 3 (stream consumption + extraction e2e): Slice 1c e2e test + smoke run.
- AC 4 (process-group timeout kill): Slice 1c timeout test.
- AC 5 (Nix reproducible build/tests): Slice 1d `nix build`.
- AC 6 (NEW — signal interruption): Slice 1c interrupt test — SIGINT/SIGTERM
  yields `result.json(outcome=interrupted)` + dead process group.
- AC 7 (NEW — artifacts + result.json): Slice 1c — run-dir bundle
  (`prompt.txt`, `stream.jsonl`, `stderr.log`, `cast-agent.pid`, `result.json`)
  produced with the verdict fields on completed/failed outcomes.

## Notes / risks to confirm during execution

- RESOLVED (see "Pre-implementation findings" above): opencode self-exit (C6),
  `drop(stdin)` load-bearing, `extract_result` last-text rule, run-log writer
  strategy. The load-bearing must-fixes (C5 env, C1 bounded joins, B2c stdin
  task, B2a re-wait, guard split, persist-before-spawn, atomic result.json) are
  folded into Slice 1c above.
- claudecode `--output-format stream-json` may require `--verbose`; pi JSON
  event shape unverified — confirm both against the design-context headless
  references before finalizing their extractors (do not guess the field names).
  (Consider opencode-only MVP + claudecode/pi as fast-follow — a PM call.)
- `result.json` per-harness field populatability (esp. `final_message` for
  claudecode/pi) is unverified beyond opencode — start with opencode, keep the
  schema tolerant of `null` fields.
- Run-dir retention: MVP keeps everything, no GC. Note the unbounded-tmpfs
  growth risk (an orchestrator spawning many subagents can accumulate large
  `stream.jsonl` firehoses); add a cleanup knob later if needed.
- macOS (second target platform) is UNEXAMINED: killpg / SIGCHLD reaping differ
  and the ~16KiB default pipe buffer makes the B2c stdin deadlock likelier —
  verify the supervisor there before claiming cross-platform support.
