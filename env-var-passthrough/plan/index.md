---
status: closed
refs: .cue/master/task/env-var-passthrough.md
---
# Master plan: host env var passthrough

Implements: task/env-var-passthrough.md

> **Superseded 2026-08-23 — closed.** All four phases were implemented
> and reviewed on `feat/env-passthrough` (31a8e71..d86c37f), but the
> single-key design this plan describes was then pivoted for 0.2.0:
> base `env_passthrough` + `extra_env_passthrough`, and zero-trace
> removal of the hardcoded per-agent forwarding. The pivot design and
> handover state live on the task card; a new plan will be drafted
> from it. This plan is retained for history: its constraints,
> trust-boundary analysis, and rejected alternatives (values in
> Config, `CAST_ENV__` prefix, figment::Jail, admerge) remain valid
> and carry over to the pivot.

## Problem

Users want to reuse a host-side secret (e.g. `GH_TOKEN`) inside a cast
container without persisting it to `cast.env` on disk. The mechanism must
not become a hole in cast's config-approval security model: an agent
running in a container can edit workspace files (`.envrc`, `cast.json`),
so any channel it can use to move host data into the container must be
gated by approval.

## Constraints discovered

1. `Env::prefixed("CAST_").split("__")` is merged into `Config`
   (`config/loader.rs:42`), and the approval hash is
   `sha256(canonical_root || serde_json(Config))`
   (`config/approval.rs:22-34`). **Env vars therefore already participate
   in approval today.** `CAST_MEMORY=4g` already trips
   `ApprovalStatus::Changed`.
2. `ApprovalStore` persists `approved_config: serde_json::Value` in
   cleartext to `~/.local/share/cast/approved_configs.json`
   (`config/approval.rs:94, 200`). Anything hashed is also written to disk
   and printed by `cast config diff`.
3. figment's `merge` (`Order::Merge`) replaces arrays wholesale and only
   recurses into dicts (`coalesce.rs:30-32`). Array-valued config keys do
   not merge across global and project files.
4. `docker run -e NAME` (no `=`) reads the value from docker's own
   environment, so the value need never appear in cast's argv.

## Chosen approach

An approval-gated allowlist of variable **names**; values are read from
the host environment at run time and never stored.

```rust
// config/schema.rs
#[serde(default)]
pub env_passthrough: Vec<String>,
```

```json
{ "env_passthrough": ["GH_TOKEN"] }
```

Rationale, per constraint:

- Names live in `Config`, so they are hashed, diffed, and gated by the
  existing `ApprovedConfig` type. An agent adding a name to `cast.json`
  causes a hard error on the next `cast run`. No new security machinery.
- Values stay out of `Config`, so no secret reaches the approval store,
  and rotating a token never forces re-approval. (Forcing re-approval on
  every rotation would train users to rubber-stamp `cast config allow`,
  degrading approval for every other setting.)
- A plain list of names rather than a map of name to bool: a map invites
  the misreading "set `GH_TOKEN` to `true`", which is unacceptable
  ambiguity for a security-relevant key. figment replaces list-valued
  keys wholesale, so a project config overrides the global allowlist
  rather than extending it — desirable, because the effective allowlist
  stays auditable from a single file and the failure mode is fail-closed.
- Emitting `-e NAME` (valueless) keeps the secret out of argv and
  therefore out of `ps` for other host users. No spawn-site plumbing is
  needed: `DockerClient` does not customise the child environment, so
  docker inherits cast's own env.

## Alternatives rejected

- **`CAST_ENV__` prefix, deliberately outside `Config`** (the original
  design). Exempts the channel from approval entirely; an agent editing
  `.envrc` could inject `CAST_ENV__X=$(cat ~/.ssh/id_rsa)` with no
  prompt.
- **A map of name to bool.** Merges across global and project, but reads
  like "set this variable to `true`" and makes the effective allowlist
  the union of two files rather than the contents of one.
- **`figment::Jail` for testing the global config path.** Mutates
  process-global env and cwd; its lock only serialises jail-against-jail,
  not against the other tests running concurrently in the same binary.
  Replaced by threading an explicit global path through
  `load_config_with_global`.
- **Folding values into `Config`.** Writes secrets to the approval store
  in cleartext, leaks them via `cast config diff`, and re-triggers
  approval on every token rotation.
- **Switching `loader.rs` to `admerge`** to make arrays concatenate.
  Changes merge semantics for every existing `Vec` field at once and
  makes global `forbidden_paths` entries impossible to remove. Out of
  scope.

## Trust boundary

Approval gates which *names* cross into the container, not what the host
shell put in them. `.envrc` overwriting an already-approved `GH_TOKEN`
with sensitive data is not caught — but an attacker who can run `$(...)`
in your shell already has host code execution and can exfiltrate without
involving cast. (direnv independently requires `direnv allow` after any
`.envrc` change.) The docs must state this plainly rather than imply
approval sanitises values.

## Phases

1. **Schema and precedence.** Add `env_passthrough` to `Config`; verify
   global/project precedence; verify approval sensitivity.
2. **Pure helper.** `build_env_passthrough` in `dev/env_file.rs`: takes
   the allowlist and a host env map, returns args plus name/value pairs.
   Skips disabled, unset, and invalid names; sorted output.
3. **Wiring.** Call from `build_docker_run_flags` after the `--env-file`
   block; apply `Command::env` at the docker spawn site.
4. **Docs.** `crates/cast/docs/config/env-overrides.md`.
