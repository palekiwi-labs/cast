---
status: complete
refs:
  - /home/pl/code/palekiwi-labs/cast/.cue/nix-native-harnesses/plan/index.md
  - /home/pl/code/palekiwi-labs/cast/.cue/nix-native-harnesses/trace/1784913729-6950cc1/diff-review-fixes.md
---

# Phase 7 — Diff-review fixes

## Foreword

Post-review cleanup for the nix-native-harnesses branch
(`feat/universal-container`). A diff-reviewer-opus review, independently
verified by consultant-opus (all six findings ACCURATE), surfaced two
should-fix items and four nits. No blocking issues; build/clippy/227 tests are
already green. Findings trace:
`.cue/nix-native-harnesses/trace/1784913729-6950cc1/diff-review-fixes.md`.

This phase applies the fixes TDD-first where behavior changes (items 1 and 2),
and as mechanical edits for the nits. Group into small, verifiable commits.

Key source anchors (verified):

- `crates/cast/src/dev/run.rs` — `run_agent` (151), universal inline (221-232),
  `run_in_container` (123-137), `resolve_run_opts` (236-261), stale doc (85-86),
  tests (610, 632).
- `crates/cast/src/dev/exec.rs` — `exec` (46), `run_in_container` call (105).
- `crates/cast/src/dev/universal/volumes.rs` — `build_universal_run_args` (33),
  shared volumes (14-23), config-mount union (46-48).
- `crates/cast/src/dev/opencode/mod.rs` — `extra_run_args` (34-69),
  `build_data_volume_args` (100-109).
- `crates/cast/src/dev/global_flake.rs` — `scaffold_if_missing` (21-31).
- `crates/cast/src/dev/image.rs` — constants (13-14).
- `crates/cast/src/config/schema.rs` — `validate()` (178-180), test (327).
- root `flake.nix` — stale comment (17), `templates.global.path` (20).

## Steps

### Item 1 — Converge `cast exec` onto the universal mount topology

Decision needed at implementation time: converge exec onto the universal path
(preferred — matches the "universal substrate" goal) vs. document the divergence
as intentional. Plan assumes convergence.

- [x] RED: add a test asserting `cast exec`'s run-args use the shared
      `{ns}-cache`/`{ns}-local` volumes + the union of all agents' config mounts
      (mirror the existing `build_universal_run_args` coverage), currently failing
      because exec uses `extra_run_args`.
- [x] GREEN: route `run_in_container` (or the exec call site) through
      `build_universal_run_args(all_agents(), ...)` instead of
      `agent.extra_run_args`, matching `run_agent`.
- [x] Verify no double `prepare_host` / no regression in the run path; confirm
      `cast shell` unaffected (still a docker exec into a running container).
- [x] `cargo test -p cast` green; clippy + fmt clean.
- [x] Commit + cue-log.

### Item 2 — Hoist scaffold out of `resolve_run_opts`

- [x] RED: a `resolve_run_opts` unit test must NOT write to `$HOME` (assert no
      side effect / inject a temp home, or prove scaffold no longer runs here).
- [x] GREEN: remove the `scaffold_if_missing(dirs::home_dir())` call from
      `resolve_run_opts`; move it into a `scaffold_global_flake()` helper called
      by `run_agent` (and `exec`) before flake detection so the
      scaffold-then-detect ordering is preserved.
- [x] Confirm the two existing tests no longer touch real `$HOME`; add coverage
      (`test_resolve_run_opts_does_not_scaffold_global_flake`).
- [x] `cargo test -p cast` green; clippy + fmt clean.
- [x] Commit + cue-log.

### Item 3 — Fix stale doc comment (run.rs:85-86)

- [x] Already resolved during Item 1: the comment now links the real
      `[run_in_container]` / `[run_agent]` — no dangling reference remains.

### Item 4 — Fix wrong asset path in flake.nix comment (flake.nix:17)

- [x] Corrected `crates/cast/assets/global-flake.nix` →
      `crates/cast/assets/global-flake-template/flake.nix`.

### Item 5 — Revert unnecessary `pub(crate)` (image.rs:13-14)

- [x] Changed `IMAGE_BASE`/`CAST_VERSION` back to private `const`.

### Item 6 — Resolve `Config::validate()` no-op (schema.rs:178-180)

Decision: dropped the no-op `validate()` (never wired into `load_config`) plus
its test and the now-unused `anyhow::Result` import. Re-introduce when a real
invariant appears.

- [x] Dropped `validate()`, its test, and the unused import.

### Optional — Security note (docs)

- [x] Added a note to `docs/nix/flake-integration.md`: `accept-flake-config`
      auto-trusts future flakes' declared substituters/keys non-interactively
      (signatures still verified against declared keys).

### Wrap-up

- [x] Nits 4-6 + docs note shared one commit; clippy/fmt/tests clean.
- [x] `cargo test -p cast`, `cargo clippy -p cast`, `cargo fmt --check` all clean.
- [x] cue-log the phase-7 completion.
