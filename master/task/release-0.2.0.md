---
title: Ship 0.2.0
status: open
priority: high
refs:
- .cue/master/task/env-var-passthrough.md
- .cue/master/task/design-repo-defined-shells.md
kind: coord
---
# Ship 0.2.0

Coordinate scope, stories, and release mechanics for the 0.2.0 tag.

## Context

- `v0.1.0` is the only existing tag. It was cut before
  `feat/universal-container` merged; that work already bumped
  `crates/cast` to `version = "0.2.0"` in Cargo.toml, so the crate
  version is ahead of the release. `cast-mcp-client` is still `0.1.0`.
- 0.2.0 is a **breaking release**. 0.1.0 was experimental; we carry
  breaking changes with strictly no backward compatibility, no
  deprecation shims, and no trace of superseded behaviour in the
  release. Users migrate by reading the release notes.

## Stories tagged `0.2.0`

Exactly two task cards carry `tag: 0.2.0` today:

- `task/env-var-passthrough` (build, in-progress): env passthrough
  allowlist. Pivoting from a single `env_passthrough` key to
  base + `extra_env_passthrough`, and removing the hardcoded
  per-agent `PASSTHROUGH_VARS` forwarding (opencode, claudecode, pi)
  entirely — breaking; provider keys must move into user config.
- `task/design-repo-defined-shells` (design, in-progress):
  repository-defined agent/project shells. Design must converge with
  the user first, then spawn build task(s), which this task tracks.
  Candidate direction (`global_shell` accepting flake refs) may also
  be breaking.

## Purpose

1. Agree what is in and out of 0.2.0 before tagging.
2. Track story progress and sequencing (design before build).
3. Own release mechanics: release notes with breaking changes called
   out, version audit, tag `v0.2.0`, push.

## Out of scope

- Implementation of any story; that lives in the story tasks and
  their spawned children.

Plan: `.cue/release-0.2.0/plan/index.md`
