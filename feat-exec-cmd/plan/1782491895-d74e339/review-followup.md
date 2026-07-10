---
status: complete
---
# Executive Plan: Review Follow-up — publish bool + S3 + minor fixes

Ref task: `.cue/master/task/1782484874-20201aa/exec-cmd.md`
Branch: `feat/exec-cmd`

## Foreword

This plan addresses the actionable findings from the Opus code review
(`trace/.../review-opus.md`). Three groups of changes, each committed
separately per the TDD + granular-commit protocol.

Decisions confirmed with the user:

- **M1 (--name collision):** No code change. Tighten doc comment on
  `resolve_container_name` to state that `--name` opts out of the
  collision-avoidance guarantee.
- **S7 / PublishPort (--publish foot-gun):** Delete `PublishPort` enum
  entirely. Replace `Option<PublishPort>` with `bool` everywhere.
  `--publish` / `-p` becomes a plain boolean flag. Fixed-port support
  is a config concern (`cast.json` `port` key), not a CLI flag.
- **S3 (run/exec duplication):** Extract a shared
  `run_in_container(agent, config, run_opts, container_name, image_tag,
  cmd)` helper that both `run_agent` and `exec` delegate to. Keep the
  user-facing nix-devshell announcement in `run_agent` only.

Commits planned:
1. `refactor: replace PublishPort enum with bool`
2. `refactor: extract run_in_container from run_agent and exec`
3. `fix: address minor review nits`

---

## Steps

### Slice 1 — Replace PublishPort with bool

- [x] 1.1 — Write failing test: `--publish` bare flag parses to `publish: true`
             in both `RunFlags` and `ExecFlags` (RED)
- [x] 1.2 — Delete `PublishPort` enum + `FromStr` impl from `run.rs`
- [x] 1.3 — Change `RunOpts.publish: Option<PublishPort>` → `bool`
- [x] 1.4 — Change `SessionFlags.publish: Option<PublishPort>` → `bool`
- [x] 1.5 — Update `build_docker_run_flags`: match → `if opts.publish`
- [x] 1.6 — Update `RunFlags` and `ExecFlags` in `cli.rs`:
             remove `num_args`, `default_missing_value`, `value_name`;
             change type to `bool`; update help text
- [x] 1.7 — Update all callers / test fixtures (remove `PublishPort::Auto`,
             `PublishPort::Fixed`, `Some(...)` wrapping; change to `bool`)
- [x] 1.8 — Delete `test_build_docker_run_flags_publish_fixed` and
             `test_exec_publish_auto_parsed`; add/update tests for bool shape
- [x] 1.9 — Run `cargo test` + `cargo clippy` → GREEN; commit

### Slice 2 — Extract run_in_container

- [x] 2.1 — Add `run_in_container` signature to `run.rs` (or a new
             `dev/runner.rs`) with a failing call from a test (RED)
- [x] 2.2 — Implement `run_in_container`; factor common body out of
             `run_agent` (GREEN)
- [x] 2.3 — Update `exec.rs` to call `run_in_container`; verify exec
             tests still pass
- [x] 2.4 — Run `cargo test` + `cargo clippy` → GREEN; commit

### Slice 3 — Minor nits

- [x] 3.1 — `container_name.rs` doc: update parenthetical "headless" →
             "ephemeral"; tighten `--name` collision note
- [x] 3.2 — Drop dead `num_args = 1..` annotation from `ExecAgent` variants
             (M2); add comment that runtime `bail!` is the enforcement point
- [x] 3.3 — Fix `--raw` doc string: "skips command wrapping only; /nix
             is still mounted and the Nix daemon is still started"
- [x] 3.4 — Rename/fix `test_build_exec_cmd_non_raw_splits_cmd_args`
             (N3 — currently a duplicate; make it actually test arg splitting
             with a flake config)
- [x] 3.5 — Add `test_exec_run_name_differs_from_interactive_run_name`
             (S11 — locks the collision-avoidance invariant)
- [x] 3.6 — Add `test_exec_empty_cmd_returns_error` in `exec.rs` (S9)
- [x] 3.7 — Run `cargo test` + `cargo clippy` → GREEN; commit
