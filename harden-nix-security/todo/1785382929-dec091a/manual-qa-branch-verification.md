---
status: open
priority: high
refs:
- .cue/master/task/harden-nix-security.md
- .cue/harden-nix-security/plan/index.md
---
# Manual QA: verify `feat/harden-nix-security`

End-to-end verification of every change on the branch. All four commits are
already merged onto `feat/harden-nix-security` (checked out at repo root):

- `6dbb098` restrict nix daemon trusted-users to root
- `9bf2972` seed default cast.json with numtide cache on first run
- `d798108` wire global cast.json scaffold into run and exec
- `6922b08` set accept-flake-config=false in dev container
- `dec091a` docs: correct numtide cache model

## Why a rebuild + daemon restart is mandatory

Two changes are NOT picked up by just rebuilding the binary:

- `accept-flake-config = false` is baked into the dev image at `docker build`
  time (`Dockerfile.dev`). The dev image must be rebuilt.
- `trusted-users = root` is injected via `NIX_CONFIG` at daemon container
  *start* (`nix_daemon/daemon.rs`). `ensure_running` reuses any existing
  `cast-nix-daemon` container, so the OLD daemon keeps the OLD config until it
  is stopped and recreated.

## Pre-flight

- [ ] Confirm branch + clean tree: `git -C /home/pl/code/palekiwi-labs/cast status`
      -> on `feat/harden-nix-security`, nothing to commit.
- [ ] Build the branch binary: `cargo build -p cast`.
- [ ] Automated suite still green: `cargo test -p cast --lib`
      (expect 229 passed), `cargo clippy -p cast --lib` (clean).

## Part A - trusted-users hardening (commit 6dbb098)

- [ ] Stop the running daemon so new config applies:
      `cargo run -p cast -- nix-daemon stop` (or `docker stop cast-nix-daemon`).
- [ ] Confirm it is gone: `docker ps --filter name=cast-nix-daemon` -> empty.
- [ ] Start a normal cast session with the freshly built binary (this recreates
      the daemon container).
- [ ] Inspect the LIVE daemon config:
      `docker exec cast-nix-daemon sh -c 'echo "$NIX_CONFIG"'`
      -> MUST contain `trusted-users = root` and `allowed-users = *`, and MUST
      NOT have a trailing `*` on `trusted-users`.

## Part B - Dockerfile accept-flake-config=false (commit 6922b08)

- [ ] Rebuild the dev image so the new nix.conf is baked in:
      `cargo run -p cast -- build --force` (or the project's normal rebuild).
- [ ] Verify the baked system config inside the dev image/container:
      `docker exec <dev-container> cat /etc/nix/nix.conf`
      -> MUST contain `accept-flake-config = false`.
- [ ] Confirm no interactive "do you want to accept flake configuration"
      prompt blocks a non-interactive `cast run`/`cast exec`.

## Part C - default cast.json seeding (commits 9bf2972 + d798108)

Simulate a FRESH user without destroying your real config.

- [ ] Move your real global config aside:
      `mv ~/.config/cast/cast.json ~/.config/cast/cast.json.bak`
      (skip if you have none).
- [ ] Run `cast run <agent>` (or `cast exec ...`) once.
- [ ] Confirm the loud notice was printed:
      "Created global cast config at ~/.config/cast/cast.json".
- [ ] Confirm the file now exists and contains the numtide entries:
      `cat ~/.config/cast/cast.json` -> `nix_extra_substituters` includes
      `https://cache.numtide.com`, `nix_extra_trusted_public_keys` includes
      `niks3.numtide.com-1:DTx8wZduET09hRmMtKdQDxNNthLQETkc/yaX7M4qK0g=`.
- [ ] Idempotency: run `cast run <agent>` again -> NO "Created ..." notice,
      file unchanged (compare mtime/contents).
- [ ] Non-overwrite: edit the seeded file (e.g. add a comment field), run
      again -> your edit is preserved, not clobbered.

## Part D - end-to-end cache provisioning (the whole point)

With the fresh seeded cast.json and the hardened daemon running:

- [ ] Confirm the daemon's generated server-side config carries the numtide
      cache: `docker exec cast-nix-daemon sh -c 'echo "$NIX_CONFIG"'`
      -> includes `cache.numtide.com` + the numtide key.
- [ ] Fetch a harness that lives in the numtide cache (e.g. inside the session
      run the harness, or `nix run llm-agents.nix#herdr`).
- [ ] Expected: it DOWNLOADS from `cache.numtide.com` (binary-cache hits), does
      NOT build from source (no multi-minute compile).
- [ ] Client-side `warning: ignoring untrusted substituter` / `ignoring
      untrusted flake configuration` warnings may still appear (client is
      non-trusted) but MUST NOT prevent the fetch - the daemon does the
      substitution. Note whether they are gone/reduced vs before.

## Part E - regression guard (negative case, optional but recommended)

- [ ] Temporarily clear the cache arrays in the seeded cast.json
      (`nix_extra_substituters: []`, keys `[]`), restart the daemon, and confirm
      the SAME harness now builds from SOURCE. This proves caches genuinely flow
      through cast.json (not some residual path). Then restore the numtide
      entries and restart the daemon.

## Teardown

- [ ] Restore your real config:
      `mv ~/.config/cast/cast.json.bak ~/.config/cast/cast.json`
      (if you backed one up).
- [ ] Restart the daemon so your normal config is live again.

## Record for each part

- Exact commands run and pass/fail.
- Any nix warnings printed verbatim.
- Whether caches were used (downloads) or bypassed (source builds).

## Sign-off

- [ ] All parts A-D pass -> update Evidence on the `harden-nix-security` task
      acceptance criteria and set the task `status: complete`.
