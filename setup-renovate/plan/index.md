---
status: complete
---
# Master Plan: Set up Renovate bot

## Overview
Configure Renovate Bot for the `cast` repository to manage automated updates for Rust Cargo dependencies and Nix flake inputs with proper grouping, scheduling, and conventional commit conventions.

## Architecture & Configuration Decisions
- **Config File Location**: `.github/renovate.json` adhering to Renovate bot schema.
- **Preset**: Extend `config:recommended`.
- **Managers**:
  - `cargo`: For Rust workspace crates (`crates/cast`, `crates/cast-mcp-client`) updating `Cargo.toml` and `Cargo.lock`.
  - `nix`: For `flake.nix` and `flake.lock` (updating `nixpkgs`, `fenix`, `flake-utils`).
  - `github-actions`: For GitHub Actions workflows in `.github/workflows`.
- **Grouping & Scheduling**:
  - Schedule: Weekly before 6am on Mondays UTC.
  - Rust/Cargo: Group non-major dependencies; isolate major updates.
  - Flake inputs: Group weekly flake lock updates.
  - Semantic commit messages: Conventional commits (`chore(deps)` / `feat(deps)`).
  - PR labels: `dependencies`.

## Implementation Slices

- [x] **Phase 1: Research & Configuration Drafting**
  - [x] Determine full list of dependencies and package managers in `cast`.
  - [x] Draft `.github/renovate.json` with recommended presets, semantic commits, and custom package rules.
  - [x] Define specific package rules for Cargo workspace dependencies and Nix flake inputs.

- [x] **Phase 2: Configuration Validation**
  - [x] Validate syntax and schema of Renovate configuration file.
  - [x] Verify alignment with project CI/build constraints and Nix sandbox requirements.

- [x] **Phase 3: Final Verification & Documentation**
  - [x] Run test suite (`cargo test`, flake checks).
  - [x] Record milestones and log entries in cue.
  - [x] Review against task acceptance criteria.
