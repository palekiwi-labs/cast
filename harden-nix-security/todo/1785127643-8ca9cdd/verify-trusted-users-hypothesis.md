---
status: complete
priority: high
refs: .cue/master/task/harden-nix-security.md
---
> Outcome (verified): hypothesis CONFIRMED. Manual QA ran
> `nix run llm-agents.nix#herdr`, which fetched from cache.numtide.com via the
> daemon despite client-side "ignoring untrusted substituter" warnings. The
> daemon (root, trusted) performs substitution using its own server-side config
> generated from `cast.json` (`nix_extra_substituters`/keys), so caches work for
> the non-trusted dev user. Corollary: the flake `nixConfig` cache path is dead
> under a non-trusted user, which motivated the cast.json seeding work
> (committed on feat/harden-nix-security). See log entries and
> plan/index.md for the resulting design.

# Manual QA: verify `trusted-users` tightening

Validate the hypothesis that the dev container user only needs
`allowed-users` (connect-only), not `trusted-users` (privilege
elevation), because substituters + public keys are baked into the
daemon's server-side config.

## Change under test

Worktree: `.worktrees/harden-nix-security` (branch `feat/harden-nix-security`).
Commit: `feat: restrict nix daemon trusted-users to root`.

`crates/cast/src/nix_daemon/config.rs` now emits:

```
trusted-users = root
allowed-users = *
```

instead of `trusted-users = root *`.

## Why a fresh daemon is required

The nix.conf is injected via the `NIX_CONFIG` env var at *container
start* (`daemon.rs:53`), not baked into the image. `ensure_running`
reuses any container already named `cast-nix-daemon` and only rebuilds
the image if it is missing. So rebuilding the binary is not enough --
the OLD daemon container keeps the OLD config until it is stopped.

## Steps

1. Build the tightened binary from the worktree:
   - `cargo build -p cast` (run in `.worktrees/harden-nix-security`).

2. Stop the currently running daemon so the new config takes effect:
   - `docker stop cast-nix-daemon`  (it was started with `--rm`, so
     this removes it), OR
   - `cargo run -p cast -- nix-daemon stop`.
   - Confirm it is gone: `docker ps --filter name=cast-nix-daemon`.

3. Start a normal cast dev session using the freshly built binary.
   The new daemon container will start with the tightened config.
   - Confirm the live config:
     `docker exec cast-nix-daemon sh -c 'echo "$NIX_CONFIG"'`
     -> expect `trusted-users = root` and `allowed-users = *`,
     and NO trailing `*` on trusted-users.

4. Baseline cache access (should PASS):
   - Inside the dev session run a `nix develop` on a project flake whose
     inputs resolve to `cache.nixos.org`.
   - Expect binary-cache hits (downloads), not source rebuilds.
   - Watch for any `warning: ignoring untrusted substituter ...`.

5. Daemon-side build (should PASS):
   - Build/realise a derivation not present in the cache.
   - Confirm it builds (daemon builds as root on the user's behalf) with
     no permission errors.

6. Custom cache -- THE REAL RISK AREA (should PASS if hypothesis holds):
   - Use a flake that references an extra cache via the user config
     `nix_extra_substituters` / `nix_extra_trusted_public_keys`
     (e.g. `~/.config/cast/nix/flake.nix`).
   - Expect the extra cache to actually be USED (downloads), not ignored.
   - Watch specifically for:
     `warning: ignoring untrusted substituter '<url>', you are not a
     trusted user` or `ignoring untrusted flake configuration setting`.

## Pass / fail criteria

- PASS: steps 4-6 all work with only `allowed-users = *`. Hypothesis
  confirmed -> promote the change out of the spike and finalize.
- FAIL: if step 6 (or any step) emits "ignoring untrusted substituter"
  or falls back to source builds, note the EXACT warning and which
  operation triggered it. That failure identifies the specific behavior
  that genuinely required `trusted-users`, which informs the real fix
  (e.g. scoping trusted-users to the resolved host UID/username instead
  of `*`).

## Notes to capture during QA

- Exact commands run and their outcome (pass/fail).
- Any warnings printed by nix.
- Whether extra/custom caches were used or bypassed.
