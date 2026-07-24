---
status: complete
priority: high
refs:
- .cue/nix-native-harnesses/plan/index.md
- .cue/nix-native-harnesses/spec/index.md
- .cue/master/task/nix-native-harnesses.md
---
# Manual QA — nix-native-harnesses (pre-merge gate)

All six implementation phases are GREEN and committed (`9224622`). Unit tests,
clippy, and fmt are clean. What remains before the task can be marked complete
and `feat/universal-container` merged is human-driven empirical verification —
the checks that a sandboxed unit test cannot perform (real containers, network,
the numtide substituter). Work top to bottom; note pass/fail + evidence next to
each item.

## 0. Build the artifacts

- [x] `cargo build -p cast` (or the nix build) produces the 0.2.0 binary.
- [x] Dev image builds: confirm it is the single shared tag
  `localhost/cast:0.2.0` (no per-agent tags, no content-hash suffix).
- [x] Confirm the image is harness-free: no opencode/pi/claudecode binaries
  baked in (e.g. `docker run --rm localhost/cast:0.2.0 which opencode` fails).

## 1. accept-flake-config sufficiency (the hard prerequisite)

This validates the whole model. Without it, non-interactive harness fetches
fail.

- [x] Confirm the dev container's `/etc/nix/nix.conf` contains
  `accept-flake-config = true` alongside `experimental-features`.
- [x] In a fresh dev container with the numtide-substituter global flake, run
  `nix develop /home/<user>/.config/cast/nix#opencode` and confirm:
  - [x] NO interactive "do you want to allow configuration setting" prompt.
  - [x] A `cache.numtide.com` cache HIT (prebuilt harness fetched, not built
    from source). Watch the fetch output / substituter lines.

## 2. First-run auto-scaffold (clean host)

- [x] On a host with NO `~/.config/cast/nix/flake.nix`, run `cast run opencode`.
- [x] The global flake is scaffolded from the embedded template with a loud
  notice; the file appears at `~/.config/cast/nix/flake.nix`.
- [x] An existing/user-modified global flake is left UNTOUCHED on a second run.

## 3. End-to-end per-harness resolution

- [x] `cast run opencode` → enters the `opencode` devShell → `opencode` binary
  resolves from the nix store and launches.
- [x] `cast run pi` → `pi` shell → `pi` resolves.
- [x] `cast run claudecode` → `claudecode` shell → `claude`/claude-code
  resolves. (Confirms the template attr name `claude-code` maps correctly.)
- [x] Verify the llm-agents.nix attribute names (opencode / pi / claude-code)
  in the shipped template match the real upstream flake end-to-end.

## 4. Per-agent container model (the retained behaviour)

- [x] Running two agents in the same project produces two distinct containers
  (`cast-opencode-<basename>-<port>` and `cast-claudecode-<basename>-<port>`)
  with different ports, coexisting without collision.
- [x] `cast shell opencode` attaches to the running opencode container and lands
  in the opencode devShell; `cast shell opencode --raw` gives a bare
  `/bin/bash` (no devShell wrapping).
- [x] `cast shell <agent>` on a NOT-running container fails with the expected
  "Dev container is not running... Run 'ocx run <agent>'" message.

## 5. global_shell override

- [x] Set `global_shell = "universal"` in `cast.json`, run `cast run opencode`,
  and confirm the container enters the `universal` devShell (all three harnesses
  on PATH). NOTE the known wrinkle: the container is still NAMED
  `cast-opencode-...` (see master note
  `note/universal-shell-and-container-identity.md`) — decide separately whether
  to ship `universal` at all.

## 6. Backward-compat config

- [x] A `cast.json` still containing the removed `agent_versions` /
  `universal_container` / `version_cache_ttl_hours` keys loads WITHOUT error
  (keys silently ignored).

## 8. Phase 7 review-fix regressions

Behavioural changes landed after the phases-1-6 QA above. Re-verify these
specific paths (`f7e8d39` scaffold hoist, `910e066` exec convergence). Unit
tests cover the arg assembly; these steps confirm real-container behaviour.

### 8a. First-run scaffold moved out of `resolve_run_opts` (Item 2)

- [x] On a clean host (no `~/.config/cast/nix/flake.nix`), `cast run opencode`
  still scaffolds the flake with the loud notice before entering the devShell
  (scaffold-then-detect ordering preserved after the hoist).
- [x] On a clean host, `cast exec opencode <cmd>` ALSO scaffolds the missing
  global flake (scaffold now runs in `exec` too, not just `run`).
- [x] A second `cast run`/`cast exec` leaves an existing flake UNTOUCHED (no
  re-scaffold, no notice).

### 8b. `cast exec` shares the universal mount topology (Item 1)

- [x] `cast exec opencode <cmd>` starts a fresh `--rm` container (not a
  `docker exec` into a running one) and the command runs.
- [x] Inside that exec container, the shared data volumes are mounted
  (`cast-cache` → `~/.cache`, `cast-local` → `~/.local`) — NOT per-agent
  `cast-opencode-cache` volumes.
- [x] Inside that exec container, the UNION of every harness config dir is
  present (`~/.config/opencode`, `~/.claude` + `~/.claude.json`, `~/.pi`) —
  identical to what `cast run` mounts.
- [x] `cast exec --raw opencode /bin/bash` still bypasses the Nix devShell
  wrapping (bare command), while non-raw exec wraps via the global flake shell.

## 9. Sign-off

- [x] All above pass → fill the "Manual QA passed" Evidence field on
  `master/task/nix-native-harnesses.md` (human attestation) and set the task
  `status: complete`.
- [x] Merge `feat/universal-container` per the release plan (0.2.0).
