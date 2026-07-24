---
status: complete
refs:
- /home/pl/code/palekiwi-labs/cast/.cue/nix-native-harnesses/spec/index.md
- /home/pl/code/palekiwi-labs/cast/.cue/master/task/nix-native-harnesses.md
---
# Nix-native harnesses & global shell selection — Master Plan

## Problem & approach

Move harness provisioning out of Docker images and into named devShells of the
global cast flake. Reuse the mount/volume/registry foundation already built on
`feat/universal-container`; delete its Docker-image-assembly machinery. Continue
on the existing `feat/universal-container` branch.

## What survives / dies from `feat/universal-container`

Verified by reading the branch (all citations `crates/cast/src/...`).

**Survives — becomes the unconditional default:**

- `dev/agent.rs:88` `Agent::config_mount_args()` — per-harness config bind
  mounts (opencode `~/.config/opencode`; claudecode `~/.claude` +
  `~/.claude.json`; pi `~/.pi`).
- `dev/universal/volumes.rs:43` `build_universal_run_args()` — union of all
  harness config mounts.
- `dev/universal/volumes.rs:17` `build_universal_data_volume_args()` — shared
  `{ns}-universal-{cache,local}` volumes, respects `volumes_namespace`.
- `dev/run.rs:219` multi-agent `prepare_host()` loop.
- Registry as a **static list**: `dev/universal/registry.rs:16` `all_agents()`.

**Dies:**

- `config/schema.rs:8` `agent_versions` field and `dev/universal/registry.rs:24`
  `resolve_included_agents()` (the filter by versions).
- `config/schema.rs` `universal_container` flag + validation.
- `dev/agent.rs:27` `dockerfile_snippet()`; `assets/Dockerfile.frag.*`;
  `dev/universal/dockerfile.rs` `assemble()`.
- `dev/universal/image.rs:37` `universal_image_tag()` / `short_content_hash`;
  `image.rs:72` `ensure_universal_image()`.
- The `if config.universal_container { } else { }` split in `dev/run.rs:194`
  and `dev/build.rs:38`.

## Key facts grounding the design

- Global-flake wrapping already exists: `dev/build_command.rs:24-33` wraps the
  container command in `nix develop /home/{user}/.config/cast/nix -c <cmd>`
  using the **default** shell. Shell selection = append `#<name>` here.
- Config loads via figment (`config/loader.rs:27-43`); top-level `Config` has no
  `deny_unknown_fields`, so a leftover `agent_versions` is silently ignored.
- Dev container writes system `/etc/nix/nix.conf` (`assets/Dockerfile.dev.*`,
  e.g. opencode:27) — currently only `experimental-features`. This is where
  `accept-flake-config = true` goes (client-side, dev container).
- Daemon: `trusted-users = root *` (`nix_daemon/config.rs:14`) already honours
  flake-declared substituters. Daemon base pinned `nixos/nix:2.34.6`
  (`assets/Dockerfile.nix-daemon:1`).

## Phases (TDD; commit at each GREEN milestone)

### Phase 1 — Config: shell selection, drop agent_versions & universal flag
- Add global-shell selection field (name of the global devShell; default =
  agent name). Wire into `dev/build_command.rs` so the global-flake layer emits
  `nix develop <global-flake>#<shell> -c ...`.
- Remove `agent_versions`, `universal_container` and their validation.
- Tests: shell name resolves (default = agent, override honoured); leftover
  `agent_versions`/`universal_container` in JSON load without error.

### Phase 2 — Make universal mounts/volumes the default path
- Delete the `universal_container` branch in `dev/run.rs` / `dev/build.rs`;
  route the single path through `build_universal_run_args` +
  `build_universal_data_volume_args` + `all_agents()` prepare_host loop.
- Delete `resolve_included_agents`.
- Tests: all supported config dirs + shared volumes mounted unconditionally;
  existing volume/mount tests adapted and green.

### Phase 3 — Strip Docker-image assembly; single plain image
- Remove `dockerfile_snippet`, `Dockerfile.frag.*`, `dockerfile::assemble`,
  `universal_image_tag`, `ensure_universal_image`.
- Dev image name becomes `localhost/cast:{cast_version}` (no content hash).
- Dev Dockerfile: **remove any harness install steps**; add
  `accept-flake-config = true` to the system `/etc/nix/nix.conf` write; keep the
  union `mkdir` of all harness config dirs.
- Tests: image tag helper returns plain versioned tag; assembly tests deleted.

### Phase 4 — Global-flake template + auto-scaffold
- Embed a default global flake template (`include_str!`) defining: `default`
  (cast tooling), per-agent shells (`opencode`, `pi`, `claudecode`), a
  `universal`/`agents` shell, `nixConfig` for `cache.numtide.com`, and a
  shell-composition reference (shared `baseInputs` → slim/extended variants).
- Also expose a nix `templates.global` output from cast's own flake.
- On `cast run`, if `~/.config/cast/nix/flake.nix` is absent, scaffold it from
  the embedded template with a loud notice. Keep `cast init` for explicit setup.
- Tests: scaffold-if-missing writes the file + notice; present flake untouched.

### Phase 5 — Version bump & release notes
- Bump `crates/cast/Cargo.toml` to `0.2.0` (also re-tags the daemon image via
  `nix_daemon/image.rs`). Keep `nixos/nix:2.34.6` pinned.
- Release notes: nix-native pivot; global flake now required (auto-scaffolded);
  `agent_versions` removed (silently ignored).

### Phase 6 — Docs
- `docs/config/*` and `docs/agents.md`: global-shell selection, template /
  auto-scaffold flow, nix-native provisioning, `accept-flake-config` rationale.

## Verification (empirical, beyond unit tests)

- **accept-flake-config**: in a fresh dev container with the numtide-substituter
  global flake, run `nix develop` and confirm (a) no flake-config prompt and
  (b) a `cache.numtide.com` cache hit. This validates the client/daemon
  two-gate reasoning before relying on it.
- **End-to-end**: `cast run opencode` on a clean host (no global flake) →
  scaffolds flake → enters `opencode` shell → opencode binary resolves from the
  nix store.

## Open decisions deferred to implementation

- Exact name of the shell-selection config field and precedence vs. any existing
  `use_flake_path`.
- Whether the universal shell is named `universal`, `agents`, or `all`.
- Embedded-template vs. `nix flake init -t` for `cast init` (ship both;
  embedded is the auto-scaffold safety net).
