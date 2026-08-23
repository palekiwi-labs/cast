---
status: complete
refs: undefined
---
# Mount ~/.agents Config Directory by Default

## Foreword

This plan implements task `.cue/master/task/mount-agents-config-dir.md`: mount
host `~/.agents` into container `/home/<user>/.agents` by default so that the
vendor-neutral cross-harness skills standard (`agentskills.io`) is available
inside every cast sandbox.

`.agents` is NOT tied to a single agent harness — it is the shared,
vendor-neutral directory natively read by OpenCode and Pi (and symlinked into
`.claude` for Claude Code). Therefore it belongs in the **universal** mount and
host-prep layer (`crates/cast/src/dev/universal/`), alongside the shared
`-cache`/`-local` data volumes, NOT in any single agent's `config_mount_args`.

Reference: `~/code/palekiwi/palekiwi/.cue/master/doc/agent-harness-config.md`
section "Mount ~/.agents by Default".

## Files to change

- NEW `crates/cast/src/dev/universal/config_dir.rs` — `.agents` path helper
  (`get_config_dir`, `ensure_config_dir`) + unit tests.
- `crates/cast/src/dev/universal/mod.rs` — register `pub mod config_dir;` and
  expose `prepare_host(opts)` (creates host `~/.agents`).
- `crates/cast/src/dev/universal/volumes.rs` — add the `.agents` bind mount in
  `build_universal_run_args` (universal step alongside data volumes) + tests.
- `crates/cast/src/dev/run.rs` — call universal `prepare_host` in
  `run_in_container` before starting the container.
- `crates/cast/assets/Dockerfile.dev` — add `/home/${USERNAME}/.agents` to the
  mkdir + chown union.
- `crates/cast/src/dev/image.rs` — add a test asserting the Dockerfile contains
  `.agents`.

## Steps (TDD, vertical slices)

- [x] Create feature branch `feat/mount-agents-config-dir`.
- [x] RED/GREEN: `universal/config_dir.rs` — `get_config_dir` + `ensure_config_dir` with unit tests.
- [x] RED/GREEN: `universal/volumes.rs` — `.agents` bind mount present in `build_universal_run_args`; add test.
- [x] Wire `universal::prepare_host` into `run_in_container` (host prep).
- [x] RED/GREEN: Dockerfile — add `.agents` dir; add image.rs test asserting `DEV_DOCKERFILE.contains(".agents")`.
- [x] Full `cargo test` + `cargo clippy` green; commit. (fmt: no new diffs;
      pre-existing drift logged in todo.)
