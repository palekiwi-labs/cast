# Review: Implementation of Review Follow-up Fixes (Opus)

**Date:** 2026-06-27
**Scope:** 3 commits on feat/exec-cmd addressing the first Opus review
  - a96ec1b refactor: replace PublishPort enum with bool
  - bc8136a refactor: extract run_in_container shared core
  - 3e356b7 fix: address minor review nits

**Verdict:** Implementation is correct, builds, 236 tests pass, clippy clean. No must-fix items.

---

## 1. Plan Fidelity

All plan boxes checked. Every commit matches the planned message.

One harmless deviation: three CLI tests added (`test_exec_publish_flag_sets_true`,
`test_run_publish_flag_sets_true`, `test_publish_absent_is_false`) vs plan's implied two.
Better coverage than planned.

Plan named tests `test_exec_run_name_differs_from_interactive_run_name` /
`test_exec_empty_cmd_returns_error` but committed as
`test_exec_auto_name_differs_from_interactive_run_name` /
`test_build_exec_cmd_empty_returns_empty`. Minor rename — see S9 note.

---

## 2. Correctness of run_in_container

Extraction is **clean and behavior-preserving**.

- `start_time`, `env`, `prepare_host`, `build_docker_run_flags`, `extend(extra_run_args)`,
  `build_run_args`, `tty_mode` match, and duration log all moved verbatim.
- `run_agent` retains: span, daemon, version/image resolution, `ensure_image`,
  `resolve_run_opts`, flake announcement, `build_command`. Correct.
- `exec` gains correct behavior it was missing (shared `extra_run_args` / `prepare_host` path).

**"session ended" log message change is acceptable.** Both callers enter distinguishing spans
(`"agent_session"` / `"exec_session"`), so log consumers can still distinguish. The shared
generic message is slightly inelegant — distinct "starting …" messages but a shared "ended" —
but harmless.

---

## 3. run_agent Post-Refactor

Sound. Flake announcement block correctly remains in `run_agent` only (post resolve_run_opts,
pre run_in_container). `exec` should not print the announcement (frequently `--raw` or one-shot).

---

## 4. Test Quality

- **N3 fix (non_raw_splits_cmd_args_with_flake):** Good. Uses `use_flake: true` +
  `project_flake_present: true`, asserts full wrapped form. Actually exercises arg-splitting.

- **S11 (exec_auto_name_differs_from_interactive_run_name):** Good. Asserts both
  `run_name != exec_name` and `exec_name.starts_with(run_name)`. Locks both invariants.

- **S9 (build_exec_cmd_empty_returns_empty):** SHOULD FIX — weakest item.
  The original S9 asked for a test that the `bail!` in `exec()` fires on empty cmd.
  The committed test instead checks that `build_exec_cmd(…, &[])` returns `[]`.
  The test's own doc comment admits "The caller (exec()) bails before build_exec_cmd is
  reached" — it's testing the wrong layer. The `bail!` path remains unit-untested.

---

## 5. Remaining Items (Not Addressed)

**Consciously scoped out (acceptable):**
- S4: token/naming split between cli.rs and exec.rs — still present, explicitly deferred.

**Should address:**
- **S9 enforcement test:** Fix the test to actually call `exec(…, cmd=vec![])` and assert
  it returns an error. Current test covers `build_exec_cmd` but not the `bail!`.
- **N8 / release note:** `cast run` no longer publishes by default. Users who relied on
  the old `publish_port: true` default will need to add `--publish`. Warrants a CHANGELOG
  entry or migration note before release.

**Nit:**
- "session ended" / "starting …" log asymmetry (minor).
- M1 part 2: no comment added about `generate_invocation_id()` collision-resistance.

---

## Summary Table

| Severity | Item |
|---|---|
| Must fix | None |
| Should fix | (a) S9 test targets wrong layer; (b) N8 release/migration note missing |
| Should fix later | S4 token/naming split (out of scope for this PR) |
| Nit | log asymmetry; invocation-id comment |
