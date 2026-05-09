# cast config allow/deny — Feature Index

## Goal

Prevent `cast run` (and any future execution commands) from starting a container
unless the current, fully-assembled configuration has been explicitly approved by
the user. Approval is performed via `cast config allow` and revoked via
`cast config deny`.

## Motivation

`cast` runs AI agents inside sandboxed Docker containers with settings drawn from
layered config files and environment variables. A malicious or accidentally altered
config could grant expanded privileges (extra volumes, relaxed resource limits,
unsafe Nix substituters). This feature adds an explicit human-in-the-loop approval
step so that no unapproved config can silently execute.

## Scope

### In scope
- `cast config allow`: assemble config, compute hash (workspace-bound), persist approval.
- `cast config deny`: assemble config, compute hash, remove approval from store.
- Gate in `cast run`: block execution if the current config+workspace hash is not approved.
- Approval store at `~/.local/share/cast/approved_configs.json`.

### Out of scope
- Approvals for `cast shell` or `cast build` (can be added later).
- Per-agent-type approval granularity.
- UI for reviewing config diff before approving.

## Key Decisions Made

- **Workspace path included in hash**: Each project directory requires its own
  approval. The same `cast.json` in two different project directories produces two
  distinct hashes.
- **All Config fields are hashed**: The full, assembled `Config` struct snapshot is
  hashed. No field is excluded. Any change (including operational fields like
  `version_cache_ttl_hours`) requires re-approval.

## Prerequisites

- `sha2` and `hex` crates are already in `Cargo.toml`.
- `serde_json` is already available for serialization.
- `dirs` crate is already available for XDG path resolution.
- `tempfile` crate is already available for atomic writes.
- No new dependencies are required.

## Related Artifacts

- Roadmap spec: `.mem/master/spec/roadmap/cast-config-allow-deny.md`
- Research report: `.mem/feat-config-allow/doc/cast-config-allow-deny-research.md`
