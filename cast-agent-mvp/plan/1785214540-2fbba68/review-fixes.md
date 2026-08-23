---
status: complete
refs:
- .cue/cast-agent-mvp/spec/index.md
- .cue/cast-agent-mvp/plan/1785127643-8ca9cdd/phase-1-mvp.md
- .cue/cast-agent-mvp/trace/1785214540-2fbba68/code-review-opus.md
- .cue/cast-agent-mvp/trace/1785214540-2fbba68/code-review-glm.md
---

# Executive Plan — cast-agent MVP Review Fixes

## Foreword

This plan addresses the actionable findings from the two-pass code review
(`trace/1785214540-2fbba68/code-review-opus.md` + `code-review-glm.md`) of the
`feat/cast-agent-mvp` branch. Both reviewers independently converged on one
critical bug (the unbounded writer-thread join defeating the C1 fallback) plus
several robustness/consistency gaps against the recovery contract. The fixes
below are scoped to items worth addressing now; items deliberately deferred are
listed at the end.

Branch: `feat/cast-agent-mvp`. TDD red-green-refactor per slice; granular
commits per the `git-commit` skill; cue-log after each commit.

## What's in scope (the items worth addressing)

- **R1 (BLOCKING, both reviewers — B1):** the writer-thread join at
  `supervisor.rs:279-283` is unbounded and hangs `supervise()` forever in the
  exact C1 escaped-grandchild scenario (a `setsid` grandchild holds the stdout
  pipe write-end after group SIGKILL). Fix the join + add the regression test
  that would have caught it (and which also exercises the never-tested
  `events_from_disk` fallback).
- **R2 (HIGH, GLM I1):** the `Err` path out of `supervise` (missing harness
  binary, reap I/O error) propagates `?` to `main`, which exits `2` (usage
  error) WITHOUT writing `result.json` — violating the "every run yields a
  verdict" contract. Map runtime failures to an Outcome and still write the
  receipt.
- **R3 (consistency, GLM N5):** `Interrupted` records the *trigger* signal in
  `exit.signal`, inconsistent with `TimedOut` (which records the child's actual
  death cause). Make `exit` always reflect the reaped child disposition and add
  a separate `interrupt_signal` field for the trigger.
- **R4 (cleanup/nits):** remove dead code, drop unused deps, add a few
  observability comments and one-line loggings.

## Slice R1 — Fix C1 writer-join hang + escaped-grandchild regression test

- [x] Add `pkgs.util-linux` to `cast-agent` `nativeCheckInputs` in `flake.nix`.
- [x] RED: `tests/supervisor_test.rs::escaped_grandchild_does_not_hang_supervise`.
      Repro needed a two-statement grandchild body (avoid exec-optimize) + a
      `sleep 0.3` in the direct child so setsid() escapes before our group
      SIGKILL races and reaps it. Confirmed RED (8s watchdog hit).
- [x] GREEN: `supervisor.rs` aborts the reader task on join timeout (releases
      its Sender, unblocking the writer thread) + bounds the writer join with
      `timeout(READER_JOIN_TIMEOUT)`. False comment fixed.
- [x] Verify `cargo test -p cast-agent` green; `cargo clippy -D warnings`; fmt.
- [x] Commit (fix: bound writer join so escaped grandchild cannot hang supervise).

## Slice R2 — Error-path verdict (every run yields result.json)

- [x] RED: `tests/orchestrate_test.rs::missing_harness_yields_crashed_verdict`.
- [x] GREEN:
      - Add `EndReason::SpawnFailed(String)` + `EndReason::SuperviseFailed(String)`.
      - In `supervise`: convert `cmd.spawn()?` to return
        `Ok(SuperviseOutput { end: SpawnFailed(e), events: vec![], pgid: None })`.
      - In `run::orchestrate`: on `Err(e)` from `supervise`, build a
        `SuperviseFailed(e)` EndReason and STILL write `result.json` + return
        `Ok(RunReport { exit_code: Crashed })`. `orchestrate` now returns `Err`
        only for the persist-before-spawn writes (genuine pre-run setup failure).
      - In `finalize::classify`: `SpawnFailed`/`SuperviseFailed` -> `Crashed`
        with `ExitInfo { code: None, signal: None }`.
      - Add `#[serde(skip_serializing_if="Option::is_none")] error_detail:
        Option<String>` to `Verdict`; populate from `SpawnFailed`/`SuperviseFailed`
        messages in `build_verdict`.
      - `main.rs`: the `Err` arm is now pre-run-setup-only; relabel the message
        to "cast-agent: setup failed: {e}" (exit 2 stays for that arm).
- [x] Verify all tests green; clippy; fmt.
- [x] Commit (fix: yield a crashed verdict on spawn/supervise failure).

## Slice R3 — `exit` field reflects child disposition (Interrupted/TimedOut)

- [x] RED: update `finalize.rs` unit tests + `interrupt_test.rs` for the new
      contract. `EndReason::TimedOut`/`Interrupted` now carry `child_status:
      Option<ExitStatus>`. `Interrupted`'s `signal` field is renamed `trigger`.
      `Verdict` gains `interrupt_signal: Option<i32>` (skip-if-none). The
      interrupt tests assert `exit.signal` == how the child DIED (SIGTERM for
      the `sleep` honors-grace cases; SIGKILL for the trap-SIGTERM case) and
      `interrupt_signal` == the trigger cast-agent received.
- [x] GREEN:
      - Change `EndReason::TimedOut` -> `TimedOut { child_status:
        Option<ExitStatus> }`; `EndReason::Interrupted { signal }` ->
        `Interrupted { trigger: i32, child_status: Option<ExitStatus> }`.
      - In `supervise`: the control plane constructs these with `child_status:
        None`; the post-teardown re-wait block fills `child_status` from
        `child.wait().await.ok()` for the TimedOut/Interrupted arms (replacing
        the current `let _ = child.wait().await;` discard). This also makes the
        reaped status authoritative for `TimedOut` (verifying our SIGKILL).
      - In `finalize::classify`: derive `ExitInfo` from `child_status` (code if
        exited, signal if killed); fall back to `signal: SIGKILL` when status is
        `None`.
      - In `build_verdict`: set `interrupt_signal` from `Interrupted.trigger`.
      - Update `supervisor_test.rs` match patterns to `EndReason::TimedOut { .. }`.
- [x] Verify all tests green; clippy; fmt.
- [x] Commit (fix: report child death signal, not trigger, for interruptions).

## Slice R4 — Cleanup + observability nits

- [x] Remove dead code `EndReason::child_signal` (+ its now-unused
      `ExitStatusExt` import).
- [x] Remove unused `tracing` + `tracing-subscriber` deps from `Cargo.toml`.
- [x] Log prompt-writer task errors via `eprintln!` on `Err`.
- [x] Log `spawn_line_writer` write errors before `break`.
- [x] `Verdict.duration_ms`: `u128` -> `u64`.
- [x] `opencode::extract_result`: `rev().filter().find_map()` so an earlier
      valid text event is used if the trailing one is malformed.
- [x] Add the N8 (ignored-signal inheritance) + I4 (reader-task panic-free)
      comments.
- [x] Verify `cargo test -p cast-agent` green (38 tests); clippy; fmt.
- [x] Commit (refactor: cleanup dead code, unused deps, and error logging).

## Deferred (not in this plan, with rationale)

- Opus I3 (signal handling during the bounded reap/join tail): windows are
  short and bounded; child already being killed. Acceptable for MVP.
- Opus I5 / Failed best-effort final_message: the design explicitly defers
  non-happy-path extraction. No change.
- extract_result array-of-parts handling / last-text re-verification: opencode
  MVP-shape only; noted in module docs.
- macOS second platform; claudecode/pi adapters; run-dir GC: out of MVP scope.

## Notes / risks

- The R3 contract change (`exit.signal` semantics for Interrupted + new
  `interrupt_signal` field) is observable; the recovery-outcome-contract note
  will need a follow-up doc update (in `.cue/`, not committed). Flagged in the
  post-commit cue-log.
- The escaped-grandchild regression test necessarily leaks an orphaned `setsid`
  grandchild that becomes a zombie in this non-reaping container — same
  environment property the existing group-death tests already tolerate. The
  test's own `setsid sleep` is SIGKILL-immune-to-cast-agent by construction
  (that's the point); the orphan persists briefly.
