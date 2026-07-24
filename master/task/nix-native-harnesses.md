---
title: Nix-native harnesses & global shell selection
status: complete
priority: high
branch: feat/universal-container
refs:
- /home/pl/code/palekiwi-labs/cast/.cue/nix-native-harnesses/spec/index.md
- /home/pl/code/palekiwi-labs/cast/.cue/nix-native-harnesses/plan/index.md
- /home/pl/code/palekiwi-labs/cast/.cue/master/task/universal-container-impl.md
- /home/pl/code/palekiwi-labs/cast/.cue/master/task/select-global-shell.md
---
# Nix-native harnesses & global shell selection

Pivot `cast` away from baking agent-harness binaries into per-agent Docker
images. Instead, provision harnesses declaratively as Nix packages exposed by
named devShells in the user's global cast flake
(`~/.config/cast/nix/flake.nix`, sourced from `numtide/llm-agents.nix`). The
global shell becomes a de-facto runtime requirement for `cast run <agent>`.

This task **merges** three previously-separate tasks into one coherent effort:

- `universal-container-impl.md` (suspended, branch `feat/universal-container`) —
  its Docker-image machinery is superseded, but its mount/volume/registry code
  is retained as the foundation here.
- `universal-container.md` — the original brief that motivated it.
- `select-global-shell.md` — the config field to pick which named global
  devShell provides the harness. This is now the *primary* selection mechanism.

Work continues on the existing `feat/universal-container` branch.

## Agreed design decisions

1. **`universal_container` boolean collapses into shell selection.** There is
   one dev image, always. Universal config-mount union + shared cache/local
   volumes become the unconditional default (not an `if universal {}` branch).
   Harness *availability* is decided by which global devShell is selected.
2. **`flake.lock` is the version source of truth.** `agent_versions` in
   `cast.json` is dropped (silently ignored on load — top-level `Config` has no
   `deny_unknown_fields`; note it in release notes). The content-hash image tag
   dies; image name becomes plain `localhost/cast:0.2.0`.
3. **Auto-scaffold** the global flake from an embedded template when missing on
   `cast run`, announced loudly. `cast init` remains for explicit setup. Also
   expose a nix `templates` output for power users.
4. **`accept-flake-config = true`** in the dev container's system
   `/etc/nix/nix.conf` — a hard prerequisite so non-interactive agents fetch
   prebuilt harnesses from `cache.numtide.com` without the flake-config prompt.
   (Daemon `trusted-users = root *` already satisfies the second gate.)
5. **Version bump to 0.2.0**, tied to this branch's merge.

## Out of scope (tracked separately)

- nix-daemon image / `/nix` volume version-skew migration — see
  `nix-daemon-volume-version-skew.md`. Keep `nixos/nix:2.34.6` pinned for now.

## Source

- Spec: `.cue/nix-native-harnesses/spec/index.md`
- Plan: `.cue/nix-native-harnesses/plan/index.md`
- Superseded: `universal-container-impl.md`, `universal-container.md`,
  `select-global-shell.md`
- Prior plan (retained history):
  `.cue/universal-container-impl/plan/1783779669-0dbafc9/universal-container.md`
- Consumer: cast-agent-design (palekiwi control center) —
  `/home/pl/code/palekiwi/palekiwi/.cue/master/task/cast-agent-design.md`
- `numtide/llm-agents.nix` — `/home/pl/code/numtide/llm-agents.nix`

## Acceptance Criteria

1. **Single dev image, no harness baked in.**
   - Verify by: inspect built image; no `opencode`/`pi`/`claude` binary present
     outside the nix store; `cast run <agent>` resolves the binary from the
     selected global devShell.
   - Evidence: Manual QA §0 (harness-free `localhost/cast:0.2.0`, no per-agent
     tags) — passed, human-attested 2026-07-24.

2. **Global devShell selection works.**
   - Verify by: config field selects a named global devShell; `cast run
     opencode` defaults to the `opencode` shell; override runs a different shell.
   - Evidence: Manual QA §3, §5 (`global_shell` override enters `universal`) —
     passed, human-attested 2026-07-24.

3. **`universal_container` boolean removed; universal mounts are default.**
   - Verify by: all supported harness config dirs and shared `cache`/`local`
     volumes mount unconditionally; no `universal_container` branch remains.
   - Evidence: Manual QA §4, §8b (run + exec share the universal topology) —
     passed, human-attested 2026-07-24.

4. **`agent_versions` dropped without breaking existing configs.**
   - Verify by: a `cast.json` still carrying `agent_versions` loads with no
     error; the field has no effect.
   - Evidence: Manual QA §6 (removed keys silently ignored) — passed,
     human-attested 2026-07-24.

5. **Auto-scaffold on missing global flake.**
   - Verify by: with no `~/.config/cast/nix/flake.nix`, `cast run <agent>`
     scaffolds it from the embedded template with a clear notice, then runs.
   - Evidence: Manual QA §2, §8a (run + exec scaffold on clean host; existing
     flake untouched) — passed, human-attested 2026-07-24.

6. **Non-interactive substituter fetch (accept-flake-config).**
   - Verify by: in a fresh dev container with the numtide-substituter global
     flake, `nix develop` shows no flake-config prompt and a `cache.numtide.com`
     cache hit.
   - Evidence: Manual QA §1 (no prompt; numtide cache HIT after the
     `niks3.numtide.com-1` key fix) — passed, human-attested 2026-07-24.

7. **Version bumped to 0.2.0.**
   - Verify by: `crates/cast/Cargo.toml` version = `0.2.0`; image tags reflect
     it.
   - Evidence: Cargo.toml/Cargo.lock/flake.nix at 0.2.0; image tag
     `localhost/cast:0.2.0` (commit 8cce186).

8. **Tests green.**
   - Verify by: `cargo test` in `crates/cast`.
   - Evidence: `cargo test -p cast` → 253 passed; 0 failed (post-Phase-7).

9. **Docs updated.**
   - Verify by: config/agents docs describe global-shell selection, the
     template/auto-scaffold flow, and the nix-native provisioning model.
   - Evidence: Phase 6 (commit 9224622); flake-integration.md security note
     (commit 5a1cea9).

10. **Manual QA passed.**
    - Verify by: human attestation.
    - Evidence: All Manual QA sections (§0-§8) passed; attested by user
      2026-07-24. Merge of `feat/universal-container` remains as the release
      step.
