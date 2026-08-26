---
status: in-progress
refs:
  - .cue/master/task/build-repo-defined-shells.md
  - .cue/design-repo-defined-shells/spec/index.md
parent: task/build-repo-defined-shells.md
---

# Master plan: repo-defined agent and project shells

Implements `.cue/design-repo-defined-shells/spec/index.md` (decisions
there are final). Tracked by task `build-repo-defined-shells`.

Baseline verification is done: every spec claim was checked against the
code (schema, build_command, run/exec/shell call paths, RunOpts probes,
scaffold modules, template, volumes mount gate, announcement, loader,
tests, docs, changelog). No deviations found.

## Slices (TDD, one commit each)

- [x] 1. Config schema: change `global_shell` semantics to full flake
     ref; add `project_shell: Option<String>`,
     `use_global_flake: bool` (default true),
     `use_project_flake: bool` (default true). Remove `use_flake`,
     `use_flake_path`. Test: legacy keys (`use_flake`,
     `use_flake_path`, `CAST_USE_FLAKE*`) silently ignored; new
     defaults load. Update `tests/config_test.rs:15`.
     (Done 3ad3310, additive only; legacy-field removal +
     silently-ignored test moved into slice 2 with the rewrite that
     orphans them, so every commit compiles.)
- [x] 2. `build_command` rewrite: refs passed verbatim to
     `nix develop <ref> -c`; layer wraps iff ref set AND switch true;
     disabled layers skip silently. Drop `agent_name` param (kills
     `unwrap_or(agent_name)` fallback); ripple to
     `Agent::build_command` trait default, `dev/exec.rs`
     (`build_exec_cmd`), `dev/shell.rs`. Rewrite the ~8
     build_command tests + run/exec/shell test helpers.
     (Done 95031e3; also removed the legacy pair + added the
     silently-ignored regression test per the slice-1 note.)
- [x] 3. Removal sweep: `RunOpts.user_flake_present` /
     `project_flake_present` and both presence probes in
     `resolve_run_opts`; `scaffold_global_flake` /
     `scaffold_global_cast_json` call sites in `run.rs` / `exec.rs`.
     Re-gate "Loading global nix devshell..." stderr announcement on
     `global_shell.is_some() && use_global_flake`.
- [x] 4. `cast config init` (flagless, global-only, never overwrites,
      partial success with notice): writes `~/.config/cast/cast.json`
      (numtide cache keys + `global_shell` =
      `~/.config/cast/nix#default`) and flake template. Absorb
      `dev/global_flake.rs` + `dev/global_config.rs`; clean module
      registrations (`dev/mod.rs`, `universal/mod.rs`).
      (Done 2df53c2, c49b63f, e12e484, c8b4085; init dispatches before
      config loading so malformed existing config is still preserved while
      a missing flake is created.)
- [x] 5. Template fold: merge empty `default` + `universal` into
      `default` = base tooling + all three harnesses; keep per-harness
      shells; drop `universal` name. Update template tests
      (global_flake.rs tests move into init).
      (Done 5d09395; verified through generated `cast config init` output.)
- [x] 6. Mount gate (`dev/universal/volumes.rs:81`): drop the
      flake.nix presence check — mount `~/.config/cast/nix` whenever
      the directory exists. New test: dir present without flake.nix
      still mounts.
      (Done 12d7490.)
- [x] 7. Docs (7 files: getting-started, nix/overview,
      nix/flake-integration, config/reference, config/env-overrides,
      agents.md, concepts.md) + 0.2.0 changelog clean-break entry
      (Unreleased section; legacy keys die loudly).
      (Done 602a93a, b91d63e; with user approval, also corrected stale
      guidance in commands/reference.md, the root quick-start, and embedded
      flake template comments. Final review found no unresolved correctness
      issues.)
- [x] 8. Clean-break rename: `global_shell` -> `sandbox_shell`,
      `use_global_flake` -> `use_sandbox_shell`, `use_project_flake` ->
      `use_project_shell` across repository source, tests, templates,
      docs, and CHANGELOG. Environment overrides become `CAST_SANDBOX_SHELL`,
      `CAST_USE_SANDBOX_SHELL`, and `CAST_USE_PROJECT_SHELL`. Preserve
      `project_shell`. Update behavioral test names/messages.
      (Done 95a0556.)

## Verification gate (each slice + final)

- [x] `cargo test` green; `cargo clippy`; `cargo fmt --check` reports only
      pre-existing formatter-version drift in untouched Rust files
- [x] Tests conform to nix-build sandbox constraints (temp dirs, no
      network, no $HOME assumptions)

## Notes / gotchas found during review

- `loader.rs` figment `Env::prefixed("CAST_")` gives the new env
  overrides for free; nothing to wire.
- Announcement exists only in `run_agent` (exec has none) — re-gate
  is a one-site change.
- Tilde refs stay verbatim; they resolve inside the container against
  the same-path `~/.config/cast/nix` mount.
- Cargo.toml already 0.2.0; changelog entry lands in `## [Unreleased]`.
