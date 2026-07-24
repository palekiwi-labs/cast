---
status: complete
refs:
- .cue/nix-native-harnesses/plan/index.md
- .cue/nix-native-harnesses/spec/index.md
---
# Phase 6 — Docs for the nix-native model

## Foreword

Executes Phase 6 (final phase) of the nix-native-harnesses master plan. No code
changes: documentation only. Updates the `cast` crate docs to describe the
nix-native harness model shipped in phases 1-5:

- Harnesses come from named global devShells, not baked Docker images.
- `cast run <agent>` selects a global devShell (default: agent name), overridable
  via the new `global_shell` config field.
- Single harness-free dev image `localhost/cast:{version}`.
- Global flake auto-scaffolded from an embedded template on first run.
- `accept-flake-config = true` in the dev container enables non-interactive
  `cache.numtide.com` harness fetches.

Source of truth verified in code: `dev/build_command.rs` (shell fragment),
`dev/global_flake.rs` (scaffold), `dev/image.rs` (single tag), `config/schema.rs`
(`global_shell`, removed fields), `assets/global-flake-template/flake.nix`.

## Steps

- [x] Update `docs/agents.md`: harnesses from devShells; drop Dockerfile/version
  resolution from the trait lifecycle
- [x] Update `docs/concepts.md`: nix-native harness provisioning + single image
- [x] Update `docs/nix/overview.md`: global-shell selection layer
- [x] Update `docs/nix/flake-integration.md`: global flake shell selection,
  auto-scaffold, template, numtide cache / accept-flake-config
- [x] Update `docs/config/reference.md`: add `global_shell`; remove
  `agent_versions` section
- [x] Update `docs/getting-started.md`: single image + auto-scaffold first-run
- [x] Update `docs/commands/reference.md`: single shared image note for run/build
- [x] `cargo test -p cast` sanity (docs-only, confirm nothing broke); commit + cue-log
