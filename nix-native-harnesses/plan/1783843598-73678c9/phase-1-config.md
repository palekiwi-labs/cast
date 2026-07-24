---
status: complete
refs: .cue/nix-native-harnesses/plan/index.md
---
# Phase 1 — Config: shell selection, drop agent_versions & universal_container

## Foreword

This plan executes Phase 1 of the nix-native-harnesses master plan. It covers:

- Adding `global_shell: Option<String>` to `Config` (default None = use agent name)
- Wiring shell name into `build_command` so the global flake ref becomes
  `{flake}#{shell}` instead of bare `{flake}`
- Removing `agent_versions` and `universal_container` from `Config` and their
  validation from `Config::validate()`
- Confirming that JSON containing those removed keys still loads without error
  (silent ignore — no `deny_unknown_fields` on top-level `Config`)

All changes stay in `crates/cast/src/config/schema.rs` and
`crates/cast/src/dev/build_command.rs`.

## Steps

- [x] Write RED test: global flake ref includes `#<agent>` by default
- [x] Write RED test: `global_shell` config override is honoured
- [x] Write RED test: `agent_versions` key in JSON is silently ignored after removal
- [x] Write RED test: `universal_container` key in JSON is silently ignored after removal
- [x] GREEN: add `global_shell: Option<String>` to `Config`; update `Default`
- [x] GREEN: add `global_shell: &str` param to `build_command`; emit `{flake}#{shell}`
- [x] GREEN: remove `agent_versions` and `universal_container` from `Config` and `Default`
- [x] GREEN: remove (or simplify) `Config::validate()` — drop the universal check
- [x] Update all call sites and existing tests to compile
- [x] Run `cargo test -p cast` — all green
- [x] Run `cargo clippy -p cast` and `cargo fmt --check -p cast` — clean
- [x] Commit
