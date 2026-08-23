---
status: open
refs: .cue/master/task/harden-nix-security.md
---
# Master Plan: Harden Nix Security (trusted-users + cache provisioning)

## Problem

`cast`'s nix daemon shipped with `trusted-users = root *`
(`crates/cast/src/nix_daemon/config.rs`), granting every connecting user -
including the non-root dev container user - full nix trust. A trusted user can
override substituters, add trusted-public-keys, import unsigned store paths,
and set restricted settings: effectively root-level trust over the shared
`/nix` store. This is a serious security hole introduced as a rushed fix for
dev-container-to-daemon socket access.

## Validated approach

### Core fix (already committed in worktree, spike validated)

Tighten the daemon config to `trusted-users = root` + `allowed-users = *`. The
dev container user can still *connect* to the daemon socket but is no longer
*privilege-elevated*. This was empirically validated: harnesses still fetch
from binary caches because the daemon (root, trusted) performs substitution
using its own server-side config.

### The cache-provisioning consequence (the real design work)

The old mechanism by which `cache.numtide.com` worked was **flake-side**: the
global-flake template declares numtide in its `nixConfig`, and
`accept-flake-config = true` in the dev container forwarded it to the daemon.
That forwarding only succeeded because the dev user was **trusted**. A decisive
experiment (empty `cast.json` cache lists on the hardened daemon) confirmed
that under a non-trusted user the flake's `nixConfig` is now **dead** - the
harness builds from source. Caches work only when declared daemon-side via
`cast.json` -> `generate_nix_conf`.

This means a fresh user with an empty `cast.json` silently regresses to
multi-minute source builds of harnesses.

## Chosen design: Option B (seed a default cast.json)

Mirror the established `scaffold_global_flake` pattern: on first run, write a
**minimal default `~/.config/cast/cast.json`** containing only the numtide
substituter + public key, with the exact same semantics (`scaffold_if_missing`:
write-only-if-absent, never overwrite, warn-and-continue on failure).

Rationale (per opus consultation + maintainer decision):
- Preserves zero-config first run; avoids a silent source-build cliff.
- The ownership objection ("cast becomes a writer of cast.json") is defanged:
  cast *already* seeds the flake in `~/.config/cast/` with identical
  semantics. This is the same pattern on a sibling file, not a new category.
- Discoverable and user-owned: the cache setting lives where users already
  edit `nix_extra_substituters`.
- Security is preserved: `approval.rs` hashes the entire `Config` (including
  `nix_extra_*`), so a hostile project-level `./cast.json` injecting a
  cache+key is gated behind `cast config allow` (human-in-the-loop) via the
  `ApprovedConfig` type. A seeded default lives in *global* config and does
  not weaken this gate.

Rejected alternatives:
- **Option A** (bake numtide into `generate_nix_conf` defaults): rejected -
  forces a harness-specific cache onto every user.
- **Option C** (document only, leave to user): rejected - failure mode is an
  invisible slow source build.

The global-flake template's `nixConfig` cache block is **kept** (per maintainer
wish): it is valid documentation and still works in external/trusted nix
environments. Only its now-false comment is corrected.

## Scope

1. **Core fix** - `config.rs` trusted-users/allowed-users (DONE, committed).
2. **Cache provisioning (Option B)** - new `scaffold_if_missing` for cast.json,
   orchestrator, call-site wiring, cross-file key-equality guard test.
3. **accept-flake-config cleanup** - `Dockerfile.dev` true -> false (explicit
   `false`, per opus, to deterministically suppress the prompt path), update
   the `dev/image.rs` test. Requires dev-image rebuild.
4. **Template + docs** - correct the false comment in the global-flake template;
   rewrite the numtide section of `docs/nix/flake-integration.md`; CHANGELOG.

## Key design decisions

- **Minimal seeded cast.json**: only the two cache arrays, no comments (strict
  JSON via figment). The "why" is documented in the flake comment + docs, not
  the JSON file.
- **Seed mirrors the flake template exactly**: numtide substituter + the
  `niks3.numtide.com-1` key. A guard test asserts byte-equality of the key
  across both embedded constants, preventing the copy-paste key-drift bug that
  already bit this project once.
- **`accept-flake-config = false`** (not deletion): deterministic
  ignore-and-warn; avoids a latent prompt that could block the interactive
  `cast shell` path.
- **`allowed-users = *`** stays for now (single-tenant). Scoping to the numeric
  host UID is a documented follow-up (the daemon container has no passwd entry
  for the host username, so UID-matching is the only viable form).

## Out of scope (follow-ups)

- Scope `allowed-users` to the resolved numeric host UID (defense-in-depth).
- Optional empty-cache stderr warning.
- The three task-card questions #2 (`/nix` ro/rw + socket mount) and #3
  (`sandbox = true`) from `harden-nix-security.md` - separate investigations.
