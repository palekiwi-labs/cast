---
title: Set up Renovate bot
status: open
priority: normal
kind: build
parent: release-0.2.0
tag: 0.2.0
---
Configure Renovate Bot for the `cast` repository to automate dependency updates across Cargo crates and Nix flake inputs.

## Context & Objectives

- Automate dependency updates for Rust Cargo workspace (`crates/cast`, `crates/cast-mcp-client`) and Nix flake inputs (`nixpkgs`, `fenix`, `flake-utils`).
- Establish a synchronized schedule and grouping rules aligned with `cue` so that shared dependencies and flake toolchains stay in sync across both projects.
- Configure `renovate.json` (or `.github/renovate.json`) with appropriate package rules (minor/patch grouping, flake lock maintenance, schedule).
- Ensure CI checks validate Renovate PRs cleanly.
