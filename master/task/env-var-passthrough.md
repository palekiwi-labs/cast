---
title: Pass host env vars into containers via env_passthrough allowlist
status: complete
kind: build
tag: 0.2.0
priority: high
refs: .cue/master/spec/index.md
---

# Pass host env vars into containers via env_passthrough allowlist

Allow users to mount environment variables into the `cast` development
container directly from the host environment, without storing them on disk
in `cast.env`.

## Source

- User request: reuse a host-side token (e.g. `GH_TOKEN`) inside `cast`
  containers without persisting secrets to `cast.env`.
- Pivot (2026-08-23, for 0.2.0): two-key base + extra design and
  removal of the hardcoded per-agent forwarding; supersedes the
  single-key design already implemented on `feat/env-passthrough`.
- Related code: `crates/cast/src/dev/env_file.rs` (`build_env_file_args`),
  `crates/cast/src/dev/run.rs` (`build_docker_run_flags`).
- Related docs: `crates/cast/docs/config/env-overrides.md`.

## Design

Two approval-gated lists of variable **names** in the cast config
schema (pivoted 2026-08-23 for 0.2.0; see Status below for what is
already implemented). Values are read from the host environment at
run time and are never stored, hashed, or logged.

```json
{
  "env_passthrough": ["ANTHROPIC_API_KEY", "OPENAI_API_KEY"],
  "extra_env_passthrough": ["GH_TOKEN"]
}
```

`env_passthrough` is the base list, intended for the global
`cast.json`; `extra_env_passthrough` holds per-project additions.
With `GH_TOKEN` set in the host shell and allowlisted, `cast run
opencode` forwards it into the container.

Key decisions:

- **Two keys, nix `extra-substituters` style.** Base
  (`env_passthrough`) + additions (`extra_env_passthrough`). Effective
  set = base ++ extra, then one filter pass (validity, reserved names,
  unset/empty, dedupe, sort) over the concatenation.
- **Per-key replacement is intended.** Both keys are plain figment
  fields; a project `cast.json` that sets either key replaces that key
  wholesale. Project config loading is scoped to its workspace, so
  replacement is exactly per-project override power and can never
  affect another project. The additive reading of "extra" (global
  extra unioned into every project) would recreate the unauditable
  global union rejected below.
- **Default empty; hardcoded forwarding removed with zero trace.**
  `cast` forwards no user-configurable host vars unless listed. The
  per-agent `PASSTHROUGH_VARS` consts and wiring
  (`dev/opencode/env.rs`, `dev/claudecode/env.rs`, `dev/pi/env.rs`)
  are deleted — no shim, no deprecation period, no migration warning
  code. Migration prose lives only in the 0.2.0 CHANGELOG: add
  provider API keys to global `cast.json` or agent containers lose
  auth.
- **No special audit surface.** Both keys are ordinary figment fields
  printed by `cast config show`; the concat stays internal to
  run-time arg construction. `CAST_ENV_PASSTHROUGH` and
  `CAST_EXTRA_ENV_PASSTHROUGH` bracketed list literals work by
  construction, approval-gated.
- **Names are config; values are not.** `env_passthrough: Vec<String>`
  is an ordinary `Config` field, so it is covered
  by the approval hash (`sha256(canonical_root || serde_json(Config))`,
  `config/approval.rs`), by `cast config diff`, and by the
  `ApprovedConfig` type gate. Values never enter `Config`.
- **Rejected: folding values into `Config`.** `ApprovalStore` persists
  `approved_config: serde_json::Value` in cleartext to
  `~/.local/share/cast/approved_configs.json`. Hashing values would (a)
  write every forwarded secret to disk, (b) invalidate approval on every
  token rotation, training users to rubber-stamp `cast config allow`, and
  (c) print secrets via `cast config diff`.
- **Rejected: `CAST_ENV__` prefix outside `Config`.** Env vars are
  already merged into `Config` (`Env::prefixed("CAST_")`,
  `config/loader.rs:42`), so they already affect approval today. Exempting
  them would carve a hole in a working security property, letting an agent
  that edits `.envrc` inject vars with no approval prompt. The prefix is
  also redundant: `CAST_ENV_PASSTHROUGH__GH_TOKEN=true` gives the same
  ad-hoc capability through the existing mechanism, approval-gated.
- **Lists, not a map of name to bool.** A map invites the misreading
  "set `GH_TOKEN` to `true`", which is unacceptable ambiguity for a
  security-relevant key; a list says exactly what it is.
- **Valueless `-e NAME`.** Emitted as `-e GH_TOKEN` (no `=`). No
  `Command::env` call is needed: `DockerClient` does not customise the
  child environment, so docker inherits cast's own env and reads the
  value from there. The secret never appears in cast's argv, so it is not
  visible in `ps` to other host users.
- Names that are unset or set-but-empty on the host, or are not valid env
  identifiers (`[A-Za-z_][A-Za-z0-9_]*`), are skipped without error, so a
  shared `cast.json` works across machines.
- Args are emitted before cast's own infra vars (`USER`, `CAST_MCP_URL`,
  ...), which remain authoritative for their names. Passthrough also
  overrides `cast.env`, but that is not positional: docker collects all
  `--env-file` entries and then appends all `--env` entries, last
  occurrence winning.
- Output is sorted by name for deterministic `docker run` argv.
- Only variable NAMES are logged, at debug level; values are never logged.

## Trust boundary

Approval gates _which names cross into the container_, not what the host
shell put in them. An attacker who can run `$(...)` in your shell already
has host code execution and can exfiltrate directly without cast. Once
the hardcoded agent lists are removed, config is the ONLY env forwarding
channel, so this statement covers all passthrough. It must be stated
plainly in the docs.

## Status and handover (2026-08-23)

- `feat/env-passthrough` at `d86c37f`: 12 commits implementing the
  ORIGINAL single-key design (schema, helper, wiring, docs, two review
  rounds, all fixes landed). Suite 261 lib + 18 + 9 green, clippy
  clean. Final opus verdict: approve-with-comments.
- The pivot is NOT implemented. It lands as NEW commits appended to
  the same branch (no rework of the existing 12). Original master and
  executive plans are closed as superseded; a new executive plan is
  to be drafted from this card. Rough commit shape: (1) second schema
  field + concat + tests, (2) delete the three agent forwarding
  modules and their dispatch wiring, (3) docs + CHANGELOG rewrite.
- Manual QA against the single-key build proved the riskiest runtime
  assumption on a real host: valueless `-e NAME` carries the value
  (inheritance), no leaks to argv/approval store/logs, empty treated
  as unset, passthrough beats `cast.env`, cast authoritative for its
  own names. The approval-gate and replace-semantics sections never
  ran; the latter is redefined by the pivot. A fresh QA checklist
  must be derived from the two-key design.
- Operational warning: a concurrent writer mutated `run.rs` /
  `loader.rs` mid-session during the last review round (surgical
  reverts of reviewed hunks; stale autosaving editor buffer is the
  leading hypothesis). Verify the working tree against git before
  trusting local file state or blaming code.
- Review nits under the pivot: N4 (unset reserved names still warn)
  unchanged; N5 (ANTHROPIC_API_KEY doc example was a no-op) dissolves
  — allowlisting provider keys becomes the required way; N6 (reserved
  list duplicated in docs prose) applies to the docs rewrite; N7
  (adding fields changes every config's serialized hash, tripping
  existing approvals) is inherent to the 0.1.0 to 0.2.0 break and
  belongs in the CHANGELOG.
