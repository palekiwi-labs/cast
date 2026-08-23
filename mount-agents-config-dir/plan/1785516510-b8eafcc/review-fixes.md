---
status: complete
refs:
- .cue/mount-agents-config-dir/trace/1785516510-b8eafcc/diff-review-opus.md
- .cue/mount-agents-config-dir/plan/1785470500-55c7cf6/agents-config-dir.md
- .cue/master/task/mount-agents-config-dir.md
---
# Review Fixes: mount-agents-config-dir

## Foreword

This plan executes the agreed fixes from the code review of
`feat/mount-agents-config-dir`. The branch was reviewed by diff-reviewer-opus
(trace: `trace/1785516510-b8eafcc/diff-review-opus.md`) and independently
verified by consultant-gemini-pro, which confirmed all findings and produced
the triage below.

Scope is deliberately bounded: only the required fix (F1) and the cheap,
high-value tightenings (F2-F5) are done here. Refactor/docs/design items
(F7, F10, F11) are logged as follow-up todos; cosmetic/rejected items
(F6, F8, F9, F12) take no action.

Implements acceptance for task `task/mount-agents-config-dir.md`. Follows the
executive plan at `plan/1785470500-55c7cf6/agents-config-dir.md` (which
delivered the initial implementation).

## Triage summary (verified)

- FIX NOW (required): F1
- FIX NOW (cheap, bundle): F2, F3, F4, F5
- DEFER (follow-up todo): F7, F10, F11
- REJECT (no action): F6, F8, F9, F12

## Steps (TDD where applicable)

- [x] **F1 — workspace-collision guard (required).** In
      `crates/cast/src/dev/universal/volumes.rs::build_agents_config_args`,
      add a guard returning `Ok(vec![])` when
      `agents_host_dir == opts.workspace.root` (mirror
      `opencode/mod.rs:40-44`). RED: add a regression test asserting the mount
      is skipped when the workspace root equals `~/.agents`. GREEN: implement
      the guard. *(commit 0355b23)*
- [x] **F2a — cover `universal::prepare_host`.** Add unit tests in
      `universal/mod.rs`: (1) creates `.agents` under a temp `host_home_dir`;
      (2) errors when `host_home_dir` is None. *(commit e7fbe87)*
- [x] **F2b — exact mount assertion.** In `volumes.rs`, add a direct unit test
      for `build_agents_config_args` asserting the exact
      `["-v", "{home}/.agents:/home/{user}/.agents:rw"]` spec (and the error
      path when host_home is None). Tighten the existing composition test's
      `.agents` assertion to the exact spec, matching sibling tests. *(commit
      e7fbe87)*
- [x] **F3 — tighten Dockerfile assertion.** In `image.rs`, change the test to
      `DEV_DOCKERFILE.matches("/home/${USERNAME}/.agents").count() == 2` with a
      concise failure message (no full-Dockerfile dump). *(commit 1b474b4)*
- [x] **F4 — relocate test under correct banner.** Move
      `universal_run_args_includes_cross_harness_agents_mount` from the
      `build_universal_data_volume_args` section to the
      `build_universal_run_args` section in `volumes.rs`. *(commit 1b474b4)*
- [x] **F5 — refresh stale doc comments.** Update the two `run.rs` doc comments
      (the session mount-topology comment ~118-124 and the `run_in_container`
      handles-list ~136-139) to mention the universal `.agents` mount and the
      `prepare_host` step. *(commit 1b474b4)*
- [x] Validate: `cargo test -p cast --lib` green; `cargo clippy --all-targets
      -- -D warnings` clean; net diff still minimal. Commit per TDD milestone.
      *(240 lib tests pass; clippy clean; no new fmt drift. 3 commits: 0355b23,
      e7fbe87, 1b474b4.)*

## Out of scope (logged as todos, NOT done here)

- **F7** — extract shared `dev::utils::{ensure_dir, host_home}` helpers (4th
  copy of config_dir pattern). Follow-up refactor todo.
- **F10** — user-facing docs: `docs/concepts.md` host-access note + `agents.md`.
  Follow-up docs todo.
- **F11** — `:rw` global instruction-dir threat surface (prompt-injection
  persistence across projects/harnesses). Decision to record, not block:
  matches `.claude`/`.pi` precedent; `ro` would break in-sandbox skill
  installs; an opt-out flag is a separate design exercise.

## Rejected (no action)

- **F6** — Dockerfile mkdir/chown inert on existing hosts: benign (bind overlay
  means host UID governs; `prepare_host` is the real guard). Commit message
  framing only.
- **F8** — `build_agents_config_args` naming: sufficiently clear in context.
- **F9** — `tempfile::tempdir()` vs `TempDir::new()`: both valid.
- **F12** — commit hygiene (b8eafcc reverts fmt noise): irrelevant under
  squash merge.
