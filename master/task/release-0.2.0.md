---
title: Ship 0.2.0
status: in-progress
priority: normal
refs:
- .cue/master/task/env-var-passthrough.md
- .cue/master/task/design-repo-defined-shells.md
- .cue/master/task/build-repo-defined-shells.md
- .cue/master/task/cast-local-json-overrides.md
- .cue/master/task/setup-renovate.md
- .cue/master/task/update-dependencies-0.2.0.md
kind: coord
tag: 0.2.0 
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

## Scope

- `task/env-var-passthrough` — complete, merged in PR #64.
- `task/cast-local-json-overrides` — complete, merged in PR #65.
- `task/build-repo-defined-shells` — complete, merged to master.
- `task/setup-renovate` — in progress.
- `task/update-dependencies-0.2.0` — in progress.

## Purpose

1. Agree what is in and out of 0.2.0 before tagging.
2. Track the remaining story to completion.
3. Own release mechanics: release notes with breaking changes called
   out, version audit, tag `v0.2.0`, push.

## Out of scope

- Implementation of any story; that lives in the story tasks and
  their spawned children.

Plan: `.cue/release-0.2.0/plan/index.md`
