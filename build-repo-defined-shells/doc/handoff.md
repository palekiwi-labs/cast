---
status: in-progress
refs:
- plan/index.md
- ../design-repo-defined-shells/spec/index.md
---
# Handoff

## Completed and committed

- Slice 4, `cast config init`: `2df53c2`, `c49b63f`, `e12e484`, `c8b4085`.
- Slice 5, template fold: `5d09395`.
- Slice 6, directory-based Nix mount gate: `12d7490`.
- Tests and clippy passed at each implementation milestone. Changed Rust files passed focused rustfmt checks. Repository-wide `cargo fmt --check` retains known pre-existing formatter-version drift.

## Current working state

Slice 7 is drafted but not committed. The working tree contains documentation-only changes in:

- `CHANGELOG.md`
- `crates/cast/docs/getting-started.md`
- `crates/cast/docs/nix/overview.md`
- `crates/cast/docs/nix/flake-integration.md`
- `crates/cast/docs/config/reference.md`
- `crates/cast/docs/config/env-overrides.md`
- `crates/cast/docs/agents.md`
- `crates/cast/docs/concepts.md`

The draft documents explicit `global_shell` and `project_shell` refs, enable switches, manual `cast config init`, the all-harness `default` template shell, directory-based mounting, removed legacy keys, and migration guidance.

## Required next actions

1. Review the uncommitted documentation diff manually. The sanity-reviewer task was cancelled and returned no result.
2. Resolve a scope question: `crates/cast/docs/commands/reference.md` still claims `cast shell` uses flake detection. It is outside the seven docs named in the approved plan. Ask whether to include this eighth correction rather than expanding scope silently.
3. Search again for stale auto-scaffolding, implicit detection, agent-name fallback, `universal`, and legacy-key claims after that decision.
4. Run appropriate final verification and commit slice 7.
5. Mark slice 7 and the verification gate complete, perform final branch review, and close the task only if no open todos remain.

## Important implementation detail

`cast config init` dispatches before normal config loading. This is intentional: malformed existing `cast.json` content must be preserved while a missing flake can still be created.
