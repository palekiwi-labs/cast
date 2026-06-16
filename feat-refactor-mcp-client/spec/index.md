# Refactor: cast-mcp-client module structure

## Goal

Reorganize `crates/cast-mcp-client/src/` to match the domain-driven,
single-concern-per-file structure used in `crates/cast/`.

## Motivation

`lib.rs` is a 1001-line monolith mixing seven distinct concerns. `main.rs`
holds both Clap struct definitions and dispatch logic. The `cast` crate
demonstrates a clean, scalable pattern that this crate should adopt.

## Scope

- Restructure `crates/cast-mcp-client/src/` only
- No logic changes, no behavior changes, no API breakage
- All existing tests must pass after the refactor
- Public API remains stable (re-exported from `lib.rs`)

## Out of scope

- Changes to `crates/cast/`
- New features
- Changes to `Cargo.toml` dependencies
- Changes to integration tests in `tests/mcp_client_test.rs`

## Success criteria

- `cargo test -p cast-mcp-client` passes (all unit + integration tests)
- `cargo clippy -p cast-mcp-client` clean
- `lib.rs` is <= 15 lines (pure `pub mod` + re-exports)
- `main.rs` is <= 25 lines (parse + `commands::run()`)
- No single file exceeds ~200 lines (excluding tests)

## Related artifacts

- Analysis: `spec/1781183559-b7ad59d/cast-mcp-client-refactor.md`
