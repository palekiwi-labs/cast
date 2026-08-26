---
title: Update dependencies for 0.2.0
status: in-progress
priority: normal
refs: .cue/master/task/release-0.2.0.md
kind: build
parent: release-0.2.0
tag: 0.2.0
---
Update Cargo dependencies and Nix flake inputs for `cast` ahead of the 0.2.0 release, aligning versions with `cue`.

## Context & Scope

- Audit and update dependencies across `crates/cast/Cargo.toml` and `crates/cast-mcp-client/Cargo.toml`.
- Align shared dependencies with `cue` (e.g. `reqwest`, `tokio`, `serde`, `clap`, `axum`, `tracing`, `figment`, `dirs`) to improve compilation cache reuse and reduce binary / closure footprint.
- Update `flake.lock` (`nix flake update`) to match the `nixpkgs` and `fenix` revisions used in `cue`, eliminating duplicated toolchain store paths in the Nix store.
- Validate with `cargo test`, `cargo clippy`, and `nix flake check` (or package builds).
