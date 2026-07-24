---
refs: /home/pl/code/palekiwi-labs/cast/.cue/master/task/nix-native-harnesses.md
---
# Nix-native harnesses & global shell selection — Spec

## Why

`cast` today bakes each agent harness (opencode, pi, claudecode) into a
dedicated Docker image and runs exactly one harness per container. This blocks
the ability for one harness to invoke another as a subprocess in the same
container — the substrate that the `cast-agent` design (in the palekiwi control
center) depends on — and forces harness versions to be pinned and installed
imperatively in Dockerfiles.

A Nix-native approach is now available: `numtide/llm-agents.nix` exposes all
supported harnesses as prebuilt Nix packages. `cast` already runs agents inside
a global Nix devShell (`~/.config/cast/nix/flake.nix`) wrapped around the
container command. By moving harness provisioning into named devShells of that
global flake, the dev image no longer needs any harness baked in, and a single
container can expose any harness — or all of them — purely by shell selection.

## What should be true

1. The dev image ships with **no harness binaries baked in**. Harnesses are
   provided exclusively by the selected global devShell.
2. `cast run <agent>` selects a **named global devShell** (default: a shell
   named after the agent), overridable via config. A shell providing all
   harnesses is the "universal" case — it is not a special build mode.
3. There is a **single dev image**. The union of all supported harness config
   mounts and the shared cache/local volumes are mounted unconditionally.
4. Harness **versions are pinned by the global flake's `flake.lock`**, not by
   `cast.json`. `agent_versions` is removed from the effective config surface.
5. A new user's first `cast run <agent>` **still works with no manual setup**:
   the global flake is auto-scaffolded from a shipped template if absent.
6. Harness substituter fetches (`cache.numtide.com`) succeed
   **non-interactively** for AI agents — no flake-config prompt.

## Scope

In scope:

- Global devShell selection config field (supersedes the standalone
  `select-global-shell` task).
- Removal of the `universal_container` boolean; universal mounts/volumes become
  the default (supersedes `universal-container` / `universal-container-impl`).
- Removal of `agent_versions` and the content-hash image tag; version bump to
  0.2.0; plain `localhost/cast:0.2.0` image name.
- Shipping a global-flake **template** (embedded for auto-scaffold + a nix
  `templates` output) that defines the named harness shells, a universal shell,
  the `cache.numtide.com` `nixConfig`, and a shell-composition reference
  pattern.
- `accept-flake-config = true` in the dev container's system `/etc/nix/nix.conf`.
- Docs for the new model.

Out of scope:

- nix-daemon image / `/nix` volume version-skew migration (separate task
  `nix-daemon-volume-version-skew.md`). `nixos/nix:2.34.6` stays pinned.
- Container sharing (one container reused by multiple `cast run` invocations).
- Selecting a *subset* of harness config mounts per shell — always mount all
  supported harnesses for now (accepted trade-off; a later config may refine).

## Prerequisites & constraints

- Depends on `numtide/llm-agents.nix` for prebuilt harness packages.
- `accept-flake-config` is a **hard prerequisite**: without it, non-interactive
  harness fetches fail and the whole model falls over.
- The daemon already sets `trusted-users = root *`, satisfying the second gate
  for honouring flake-declared substituters.

## Related

- Supersedes: `master/task/universal-container.md`,
  `master/task/universal-container-impl.md`,
  `master/task/select-global-shell.md`.
- Consumer / motivation: `cast-agent` design —
  `/home/pl/code/palekiwi/palekiwi/.cue/master/task/cast-agent-design.md`.
- Retained history: prior implementation plan at
  `.cue/universal-container-impl/plan/1783779669-0dbafc9/universal-container.md`.
