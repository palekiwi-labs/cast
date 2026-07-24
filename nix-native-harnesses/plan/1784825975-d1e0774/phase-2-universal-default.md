---
status: complete
refs: .cue/nix-native-harnesses/plan/index.md
---
# Phase 2 — Universal mounts/volumes as unconditional default

## Foreword

This plan executes Phase 2 of the nix-native-harnesses master plan. `universal_container`
was a branch guard that opted into universal mounts. Phase 2 makes those mounts
the unconditional default and deletes the branch.

Files changed:
- `crates/cast/src/dev/run.rs` — delete `if config.universal_container` branch;
  single path uses `all_agents()` prepare_host loop + `build_universal_run_args`.
- `crates/cast/src/dev/build.rs` — delete `if cfg.universal_container` branch;
  single path builds per-agent image only.
- `crates/cast/src/dev/universal/registry.rs` — delete `resolve_included_agents`
  and `validate_agent_included`; keep `all_agents()`.
- `crates/cast/src/config/schema.rs` — remove `agent_versions` and
  `universal_container` fields, their `Default` entries, and their now-obsolete
  tests; simplify `validate()` to a no-op.
- Per-agent `resolve_version` impls (opencode, pi, claudecode) — remove
  `agent_versions` lookup; return a placeholder or remove the method.

The image machinery stays per-agent for Phase 2 (Phase 3 replaces it with
the plain versioned tag).

## Steps

- [x] Write RED test: `build_universal_run_args` called for default Config — all
  agent config dirs present in extra args even with no `universal_container` flag.
  (This is already covered by volumes.rs tests; new test in run.rs proves the
  full path calls universal run args.)
- [x] GREEN: rewrite `run_agent` to always use the universal path:
  - remove `if config.universal_container` branch
  - replace `agent.extra_run_args` with `build_universal_run_args(all_agents(), ...)`
  - replace single `agent.prepare_host` with loop over `all_agents()`
- [x] GREEN: simplify `build.rs` — delete the universal branch; always build
  per-agent image.
- [x] GREEN: delete `resolve_included_agents` and `validate_agent_included` from
  `registry.rs`; delete their tests.
- [x] GREEN: remove `agent_versions` and `universal_container` from `Config`;
  update `Default`; simplify `validate()` to `Ok(())`.
- [x] GREEN: remove `agent_versions` lookup from per-agent `resolve_version`
  impls (opencode, pi, claudecode).
- [x] GREEN: delete imports that no longer compile
  (`resolve_included_agents`, `validate_agent_included`, `universal_image_tag`,
  `ensure_universal_image` from `run.rs` and `build.rs`).
- [x] Run `cargo test -p cast` — all green
- [x] Run `cargo clippy -p cast` and `cargo fmt -p cast` — clean
- [x] Commit
