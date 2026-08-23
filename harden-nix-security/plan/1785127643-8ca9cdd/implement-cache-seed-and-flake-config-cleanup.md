---
status: complete
refs:
- .cue/harden-nix-security/plan/index.md
- .cue/master/task/harden-nix-security.md
---
# Executive Plan: implement cache-provisioning + accept-flake-config cleanup

## Foreword

This plan implements the remaining slices of the `harden-nix-security` master
plan (`.cue/harden-nix-security/plan/index.md`). The core `trusted-users = root`
+ `allowed-users = *` change is **already committed** on
`feat/harden-nix-security` (`config.rs`, tests green) and empirically validated
- do NOT redo it. This plan covers: (1) Option B cache-provisioning (seed a
default cast.json), (2) the `accept-flake-config` cleanup, (3) template + docs
corrections.

All work happens in the worktree `/home/pl/code/palekiwi-labs/cast/.worktrees/
harden-nix-security` on branch `feat/harden-nix-security`. Apply TDD
(red-green-refactor) per slice, committing at each GREEN. Run
`cargo test -p cast --lib`, `cargo clippy -p cast --lib`, `cargo fmt -p cast`
before each commit. Do NOT commit `.cue/`. Do NOT touch unrelated files swept by
`cargo fmt` (revert them to keep commits atomic - this already bit us once with
import-ordering in `cli.rs` / `dev/build.rs`).

Reference facts (verified):
- Flscaffold pattern to mirror: `crates/cast/src/dev/global_flake.rs`
  (`scaffold_if_missing(home_dir)`, write-only-if-absent, returns `Ok(bool)`,
  embeds `GLOBAL_FLAKE_TEMPLATE` via `include_str!` from
  `assets/global-flake-template/flake.nix`).
- Orchestrator to mirror: `scaffold_global_flake()` in
  `crates/cast/src/dev/run.rs:259`, called from `run.rs:225` and
  `crates/cast/src/dev/exec.rs:106` (before `resolve_run_opts`).
- Config read path (never written today): `crates/cast/src/config/loader.rs`;
  global path = `home_config_dir()/cast/cast.json`. figment `Json` provider =
  strict JSON (no comments).
- accept-flake-config: `crates/cast/assets/Dockerfile.dev:20-22` (baked at image
  build); test `dev_dockerfile_accepts_flake_config` at
  `crates/cast/src/dev/image.rs:94-100`.
- Template comment to fix: `assets/global-flake-template/flake.nix:4-6`.
- Docs: `crates/cast/docs/nix/flake-integration.md:58-72`.

The numtide key (must be byte-identical everywhere): `niks3.numtide.com-1:DTx8wZduET09hRmMtKdQDxNNthLQETkc/yaX7M4qK0g=`

## Amendment (post-QA) - read before trusting the slices below

Slices 1-3 are done, but the first round of manual QA falsified a premise that
Slices 2 and 3 were built on. Two corrective slices (5 and 6) were added and
are also done. The record below is kept as-written with outcomes appended, so
the reasoning trail stays honest.

**The falsified premise:** `accept-flake-config = false` does *not* suppress
nix's flake-config approval path. `false` only means "do not auto-accept".
With an interactive TTY and no persisted answer, nix **prompts on every
devshell entry**. Declining without also answering the follow-up "permanently
mark as untrusted" prompt saves nothing, so it re-asks forever; accepting
writes `~/.local/share/nix/trusted-settings.json` and converts the prompt into
a permanent daemon warning.

**Consequences:**
- Step 3.1's decision to keep the `nixConfig` block live was wrong. The block
  is not merely inert inside the dev container, it is actively harmful. It has
  since been removed outright (Slice 6).
- Step 1.5's cross-file key guard is gone with the second copy it guarded. The
  cache trust anchor now ships in exactly one place, `DEFAULT_CAST_JSON`.
- A second, independent bug surfaced during the same QA: a trailing-slash
  mismatch in `trusted-substituters` (Slice 5).

## Slice 1 - Option B: seed a default cast.json

Done: commits `9bf2972` (module + guard) and `d798108` (orchestrator wiring).

- [x] **1.1 RED** - create `crates/cast/src/dev/global_config.rs` with a new
      `pub const DEFAULT_CAST_JSON: &str` containing a minimal strict-JSON object
      with only `nix_extra_substituters: ["https://cache.numtide.com"]` and
      `nix_extra_trusted_public_keys: ["niks3.numtide.com-1:..."]`, plus a
      `pub fn scaffold_if_missing(home_dir: &Path) -> Result<bool>` mirroring
      `global_flake::scaffold_if_missing` exactly (write-only-if-absent at
      `<home>/.config/cast/cast.json`, return Ok(false) if exists, create dir
      tree, warn-context on errors). Write the unit tests first
      (`scaffolds_when_missing`, `leaves_existing_untouched`,
      `contains_numtide_cache`) - they will fail to compile (module/function not
      wired). Declare the module in `crates/cast/src/dev/mod.rs`.
- [x] **1.2 GREEN** - implement `scaffold_if_missing` and the const so the three
      tests pass. Confirm `DEFAULT_CAST_JSON` parses as valid JSON (assert via
      `serde_json::from_str` in a test).
- [x] **1.3 RED** - add `pub fn scaffold_global_cast_json()` orchestrator in
      `crates/cast/src/dev/run.rs` next to `scaffold_global_flake` (mirrors its
      shape: resolve home via `dirs::home_dir()`, call the new
      `scaffold_if_missing`, on `Ok(true)` `eprintln!` a "Created global cast
      config at ..." notice, on `Err` `tracing::warn!`). Add a test asserting it
      is a side-effecting function (mirror the existing
      `test_resolve_run_opts_does_not_scaffold_global_flake` shape - it should
      NOT be called from `resolve_run_opts`).
      Deviation: rather than adding a standalone test, the existing
      `test_resolve_run_opts_does_not_scaffold_global_flake` purity test was
      extended to also assert cast.json is not written.
- [x] **1.4 GREEN** - wire the orchestrator. Call `scaffold_global_cast_json()`
      from the same two sites that call `scaffold_global_flake()`:
      `run.rs:225` and `exec.rs:106`, immediately after the flake scaffold.
- [x] **1.5 RED** - **later reverted, see Slice 6.** cross-file key-equality
      guard test: assert the numtide key
      substring `niks3.numtide.com-1:DTx8...` appears in BOTH
      `DEFAULT_CAST_JSON` and `GLOBAL_FLAKE_TEMPLATE`. Place it where it can see
      both consts (make both `pub(crate)` if needed, or put the test in
      `global_flake.rs` referencing the cast.json const). This test fails until
      the const exists (covered by 1.2) - write it now to lock the invariant.
- [x] **1.6 GREEN/REFACTOR** - run full `cargo test -p cast --lib`; clippy; fmt.
      Revert any unrelated fmt churn. Commit: `feat: seed default cast.json with
      numtide cache on first run`.

## Slice 2 - accept-flake-config cleanup

Done: commit `6922b08`. **Caveat:** the stated rationale ("deterministically
suppresses the interactive prompt path") is false - see the Amendment. The
change itself is still correct and retained: it stops the dev container from
auto-accepting restricted settings from *project* flakes.

- [x] **2.1 RED** - update the test `dev_dockerfile_accepts_flake_config` in
      `crates/cast/src/dev/image.rs:94-100` to assert the Dockerfile contains
      `accept-flake-config = false` (and rename it to reflect the new intent,
      e.g. `dev_dockerfile_disables_flake_config`). It currently asserts `= true`
      -> will fail.
- [x] **2.2 GREEN** - edit `crates/cast/assets/Dockerfile.dev:21`: change
      `accept-flake-config = true` to `accept-flake-config = false`. Also update
      the adjacent comment (lines 15-18) which currently calls it a "hard
      prerequisite" - that rationale is now inverted.
- [x] **2.3** - run `cargo test -p cast --lib`; clippy; fmt. Revert unrelated
      churn. Commit: `refactor: set accept-flake-config=false in dev container`.

## Slice 3 - template + docs correction

Done: commit `dec091a`. Step 3.1 was **superseded by Slice 6** - see the
Amendment.

- [x] **3.1** - **superseded.** fix the now-false comment in
      `crates/cast/assets/global-flake-template/flake.nix:4-6`. It currently
      claims `accept-flake-config = true` makes the cache honoured. New text:
      the `nixConfig` cache is honoured in standalone/trusted nix environments;
      inside cast's dev container the daemon provisions this cache server-side
      from `~/.config/cast/cast.json` (seeded on first run). Keep the `nixConfig`
      block itself live. Update the `global_flake.rs` template-content test if it
      asserts the old comment wording.
- [x] **3.2** - rewrite the "Harness fetches and the numtide cache" section of
      `crates/cast/docs/nix/flake-integration.md` (lines ~58-72) to describe the
      daemon-side provisioning model: caches come from cast.json (seeded with
      numtide by default) -> `generate_nix_conf` -> daemon; the flake's
      `nixConfig` is documentation/external-env only; `accept-flake-config =
      false`. Replace the now-moot security note.
- [x] **3.3** - add a CHANGELOG.md entry under Unreleased (Changed: nix
      `trusted-users` tightened to root; Added: default cast.json seeds numtide
      cache; Changed: `accept-flake-config` now false) following the existing
      Keep-a-Changelog style.
- [x] **3.4** - run `cargo test -p cast --lib`; clippy; fmt. Commit:
      `docs: correct numtide cache model after trusted-users hardening`.

## Slice 4 - first manual QA round (done; falsified a premise)

- [x] **4.1** - user rebuilt the dev image and reported that two warnings
      persisted on every devshell entry:
      `ignoring untrusted substituter 'https://cache.nixos.org/'` and
      `ignoring the client-specified setting 'trusted-public-keys'`.
- [x] **4.2** - diagnosed from inside the live dev container. Findings:
      - `/etc/nix/nix.conf` *was* correctly rebuilt with
        `accept-flake-config = false`; the image rebuild was not the problem.
      - `~/.local/share/nix/trusted-settings.json` in the mounted home held a
        saved `true` for the flake's `extra-substituters` /
        `extra-trusted-public-keys`, from a prompt answered in the old
        trusted-user era. Nix consults that saved list **before** honouring
        `accept-flake-config`, so the flake `nixConfig` was still applied.
      - Applying it makes `substituters` and `trusted-public-keys`
        client-overridden; `RemoteStore::setOptions` forwards only overridden
        settings, which is exactly why those two were warned about.
      - The substituter warning named only `cache.nixos.org` because of an
        exact-string mismatch, not because numtide was fine: see Slice 5.
- [x] **4.3** - user confirmed the prompt behaviour directly, falsifying the
      Slice 2/3 premise. Recorded in the Amendment above.

## Slice 5 - fix the trailing-slash mismatch (commit `6f0d98c`)

Independent latent bug, worth fixing regardless of the flake path.

The daemon matches client-forwarded substituters against `trusted-substituters`
by exact string, compensating only for a *missing* trailing slash (it will
append one, never strip one). Nix's built-in client default is
`https://cache.nixos.org/` while `generate_nix_conf` emitted
`https://cache.nixos.org`, so the default was always rejected while numtide,
being byte-identical, passed silently.

- [x] **5.1 RED** - two tests in `nix_daemon/config.rs`:
      `trusted_substituters_include_trailing_slash_variants` and
      `trusted_substituters_variant_of_slashed_entry_drops_the_slash`.
- [x] **5.2 GREEN** - `trusted-substituters` now lists every substituter in
      both spellings. `substituters` itself is unchanged. Chose the general
      form over a one-character fix to the default, since the same asymmetry
      would bite any user entry in `nix_extra_substituters`.
- [x] **5.3** - updated the two pre-existing assertions in lockstep; CHANGELOG
      `Fixed` entry; suite green, clippy clean.

## Slice 6 - remove nixConfig from the flake template (commits `85eb268`, `1d9f308`)

- [x] **6.1** - first pass (`85eb268`) shipped the block **commented out** with
      a long rationale comment and an "uncomment for standalone use" path.
- [x] **6.2** - maintainer rejected that as too verbose: the template is a
      flake for running cast's harnesses *inside* cast, and out-of-cast use is
      explicitly not a supported case. Superseded by `1d9f308`, which removes
      the block and the comment outright, leaving a two-line pointer to
      `cast.json`.
- [x] **6.3 RED** - guard tightened to a plain
      `!GLOBAL_FLAKE_TEMPLATE.contains("nixConfig")` (a substring check would
      have passed vacuously against the commented block); template then
      stripped. Verified the result still parses with
      `nix-instantiate --parse`.
- [x] **6.4 REFACTOR** - removing the second copy of the numtide key made the
      cross-file drift guard from step 1.5 pointless; deleted it along with the
      copy it guarded. `contains_numtide_cache` in `global_config.rs`
      strengthened from a prefix check to a full-key assertion, as it is now
      the sole anchor. `template_defines_expected_shells_and_cache` lost its
      cache assertions and was renamed. Docs + CHANGELOG trimmed to match.
      Net -74/+33 lines.

## Slice 7 - final manual QA (OUTSTANDING - hand back to user)

The code side of this plan is complete: 231 tests green, clippy clean, commits
`9bf2972`, `d798108`, `6922b08`, `dec091a`, `6f0d98c`, `85eb268`, `1d9f308` on
`feat/harden-nix-security`.

Host-side remediation is required because neither scaffold overwrites an
existing file and both the daemon config and the dev image are baked at
container/image creation:

- [ ] **7.1** - delete the `nixConfig` block from the real
      `~/.config/cast/nix/flake.nix` (pre-existing flakes are never
      overwritten by the scaffold).
- [ ] **7.2** - remove `~/.local/share/nix/trusted-settings.json` (holds the
      stale `true` entries that keep the flake config alive).
- [ ] **7.3** - recreate the `cast-nix-daemon` container so the new
      `trusted-substituters` from `6f0d98c` is injected via `NIX_CONFIG`.
- [ ] **7.4** - confirm: (a) devshell entry is silent - no prompt, no
      warnings; (b) a harness still fetches from `cache.numtide.com` rather
      than building from source.
- [ ] **7.5** - fresh-user simulation: move the real `cast.json` aside and
      confirm the seed fires with a loud notice and the cache still works.
