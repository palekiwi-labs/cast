# Project Log

## [8ca9cdd-dirty] Spike: tighten nix daemon trusted-users

Created worktree .worktrees/harden-nix-security (branch feat/harden-nix-security) to test the hypothesis that the dev container user needs only allowed-users, not trusted-users. TDD change to nix_daemon/config.rs: replaced `trusted-users = root *` with `trusted-users = root` + `allowed-users = *`. Committed as feat spike. Authored a manual QA todo for the user to rebuild and verify cache access still works.

- **Found:** nix.conf is injected via NIX_CONFIG env at container start (daemon.rs:53), not baked into the image; ensure_running reuses any container named cast-nix-daemon and only rebuilds the image if missing -- so the running daemon must be stopped for new config to take effect
- **Found:** cargo fmt swept unrelated import-ordering fixes in cli.rs and dev/build.rs (pre-existing on master); reverted to keep commit atomic
- **Decided:** Test the fix as a code change in a git worktree rather than a manual docker run, since the user rebuilds via cargo
- **Decided:** trusted-users = root + allowed-users = * is the candidate hardening; substituters/keys remain server-side so caches should still work for non-trusted users
- **Open:** Does a non-trusted dev user still get custom/extra caches (nix_extra_substituters), or does nix emit 'ignoring untrusted substituter'? Pending manual QA
- **Open:** If trusted is genuinely needed, consider scoping trusted-users to the resolved host UID/username instead of *

## [8ca9cdd-dirty] Tightening trusted-users kills flake-declared cache forwarding

Manual QA confirmed the trusted-users=root spike works: `nix run llm-agents.nix#herdr` fetched from cache.numtide.com via the daemon despite client-side warnings, because the user's cast.json lists numtide+palekiwi in nix_extra_substituters/keys, which generate_nix_conf bakes into the daemon's server-side config. The daemon (root, trusted) does substitution using its own config, so caches work for the non-trusted dev user. However this reframes the whole harness-cache model documented under the prior nix-native-harnesses task.

- **Found:** Under old trusted-users=root*, the dev user was TRUSTED, so accept-flake-config=true forwarded the global-flake-template's nixConfig (extra-substituters=numtide, extra-trusted-public-keys) and the daemon ACCEPTED them. That was the documented mechanism (docs/nix/flake-integration.md, Dockerfile.dev calls accept-flake-config a 'hard prerequisite').
- **Found:** Under new trusted-users=root, the dev user is NON-trusted, so the client-forwarded restricted settings are REJECTED (the two warnings). The flake's nixConfig cache mechanism is now DEAD.
- **Found:** Caches now work ONLY via cast.json -> generate_nix_conf -> daemon server-side config. The user already has numtide+palekiwi there, so herdr worked.
- **Found:** accept-flake-config=true is now largely inert for its documented purpose and just produces noise; its security note (auto-trusting future flake substituters/keys) is also moot for a non-trusted user.
- **Found:** Regression risk for fresh users: a user whose cast.json lacks numtide would now get source builds of harnesses even though the global flake declares numtide, because flake-declared caches no longer reach the daemon.
- **Decided:** trusted-users=root + allowed-users=* is validated as functionally correct given daemon-side cache provisioning
- **Open:** Where should the numtide default live so fresh users are not broken: bake into generate_nix_conf defaults (was cb03c21, reverted), scaffold into default cast.json, or leave to users?
- **Open:** Should accept-flake-config=true be removed from Dockerfile.dev, and does removing it actually silence the warnings or just change them?
- **Open:** Should the global-flake-template drop its now-non-functional nixConfig cache block?

## [8ca9cdd-dirty] Decisive test: flake nixConfig cache is dead under non-trusted user

Ran the decisive experiment on the hardened daemon (trusted-users=root, accept-flake-config still true). Result: with cast.json cache lists CLEARED, the harness builds from source; with numtide/palekiwi caches in cast.json, it downloads from cache.numtide.com. This confirms the regression is real and resolves the apparent contradiction with the nix-native-harnesses log (that finding was recorded under trusted-users=root*, i.e. a trusted user, so it never tested the non-trusted case).

- **Found:** Under trusted-users=root (non-trusted dev user), the global flake's nixConfig extra-substituters/extra-trusted-public-keys do NOT reach the daemon; caches work ONLY when declared in cast.json -> generate_nix_conf -> daemon server-side config
- **Found:** The flake nixConfig cache block is now functionally dead inside cast's dev container (still valid in external/trusted nix environments)
- **Found:** approval.rs hashes the entire Config (incl nix_extra_substituters/keys), so a hostile project ./cast.json injecting a cache+key is gated behind `cast config allow` (human-in-the-loop) via the ApprovedConfig type
- **Decided:** Cache provisioning must move to the daemon side via cast.json (the flake-forwarding path is dead under the hardened, non-trusted posture)
- **Decided:** allowed-users scoping, if pursued, must key on the numeric host UID (daemon container has no matching passwd entry for the username); treat as follow-up, * is fine for single-tenant
- **Open:** Choose B (seed default cast.json on first run) vs C (document requirement, leave to user)
- **Open:** Remove/neutralize accept-flake-config in Dockerfile.dev (+ update image.rs:94 test); decide plain-remove vs explicit =false
- **Open:** Correct the now-false flake template comment; decide whether to keep the nixConfig block live or comment it out
- **Open:** Follow-ups: cross-file key-equality guard test; optional empty-cache stderr warning

## [8ca9cdd-dirty] Plans written: cache-provisioning (B) + flake-config cleanup

Master plan (.cue/harden-nix-security/plan/index.md) and executive plan (.cue/harden-nix-security/plan/1785127643-8ca9cdd/implement-cache-seed-and-flake-config-cleanup.md) written. Decisions: Option B (seed minimal default cast.json mirroring the global_flake scaffold_if_missing pattern), fold accept-flake-config cleanup into this task, set accept-flake-config=false (explicit, per opus), keep flake nixConfig block live as documentation with corrected comment. Core trusted-users fix already committed on feat/harden-nix-security. Executive plan is TDD-structured across 3 slices + final manual QA, ready for handoff to a builder.

- **Decided:** Option B: seed a minimal default ~/.config/cast/cast.json (numtide substituter + key only, strict JSON) on first run via a new dev/global_config.rs scaffold_if_missing mirroring global_flake.rs
- **Decided:** accept-flake-config = false (explicit, not deleted) to deterministically suppress the prompt path; update dev/image.rs:94 test in lockstep
- **Decided:** Keep the flake template nixConfig block live for external/trusted environments; correct its now-false comment; rewrite flake-integration.md numtide section
- **Decided:** Cross-file key-equality guard test (numtide key identical in DEFAULT_CAST_JSON and GLOBAL_FLAKE_TEMPLATE) to prevent drift
- **Open:** allowed-users scoping to numeric host UID (follow-up)
- **Open:** empty-cache stderr warning (follow-up)
- **Open:** task-card questions #2 (/nix ro/rw + socket mount) and #3 (sandbox=true) remain unaddressed

## [8ca9cdd-dirty] Slice 1: global_config scaffold + key-guard committed

Committed 9bf2972 on feat/harden-nix-security. Added crates/cast/src/dev/global_config.rs (DEFAULT_CAST_JSON pub(crate) + scaffold_if_missing mirroring global_flake), wired module in dev/mod.rs, added cross-file numtide-key-equality guard test in global_flake.rs. Orchestrator wiring (1.3/1.4) still pending.

- **Found:** cargo fmt again swept unrelated import churn in cli.rs and dev/build.rs; reverted before commit to keep it atomic (matches prior log warning)
- **Found:** cast crate already depends on serde_json 1.0.149, used for the valid-JSON assertion
- **Decided:** DEFAULT_CAST_JSON is pub(crate) so the guard test in global_flake.rs can reference it
- **Decided:** Seeded JSON is minimal: only nix_extra_substituters + nix_extra_trusted_public_keys arrays

## [8ca9cdd-dirty] Slice 1 complete: cast.json scaffold wired into run/exec

Committed d798108. Added scaffold_global_cast_json() orchestrator in run.rs mirroring scaffold_global_flake; wired at both call sites (run.rs run_agent, exec.rs) immediately after the flake scaffold. Extended the resolve_run_opts purity test to also assert cast.json is not written. Full suite 229 green, clippy clean. Slice 1 done. Next: Slice 2 accept-flake-config true->false.

- **Decided:** Rather than a new standalone test, extended the existing does_not_scaffold purity test to cover cast.json too

## [8ca9cdd-dirty] Slice 2 complete: accept-flake-config=false

Committed 6922b08. Dockerfile.dev nix.conf now writes accept-flake-config = false (explicit) with rewritten rationale comment; renamed/updated the image.rs test to dev_dockerfile_disables_flake_config. Requires dev-image rebuild to take effect (baked at build). Next: Slice 3 template comment + docs + CHANGELOG.

## [8ca9cdd-dirty] Slice 3 complete: template comment + docs + CHANGELOG

Committed dec091a. Corrected the now-false nixConfig comment in the global-flake template (kept the block live, described daemon-side provisioning). Rewrote the numtide-cache section of docs/nix/flake-integration.md to describe cast.json -> generate_nix_conf -> daemon, accept-flake-config=false, and the seeded default. Added CHANGELOG Unreleased entries (Added: cast.json cache seeding; Changed: trusted-users hardening + accept-flake-config=false). Full suite 229 green. All 3 implementation slices done; only manual QA (slice 4) remains for the user.

- **Open:** Manual QA: rebuild dev image (accept-flake-config baked in), restart nix-daemon container (new trusted-users), simulate fresh user (move real cast.json aside) and verify seed fires + harness fetches from cache.numtide.com + warnings gone

## [6f0d98c] Root-caused the persisting untrusted-substituter warnings; fixed slash mismatch

Diagnosed the two warnings the user still saw after rebuilding the dev image. Inspected the live dev container directly (the agent runs inside it): /etc/nix/nix.conf DOES contain accept-flake-config = false, so the rebuild took effect. The real cause is a stale ~/.local/share/nix/trusted-settings.json in the mounted home recording a previous interactive 'y' for the global flake's extra-substituters/extra-trusted-public-keys. Nix checks that saved trusted-list BEFORE honouring accept-flake-config, so the flake nixConfig is still applied client-side, making `substituters` and `trusted-public-keys` client-overridden and therefore forwarded to the daemon. Committed 6f0d98c fixing the second, independent bug (trailing-slash mismatch) which is a real latent defect regardless of the flake path.

- **Found:** /etc/nix/nix.conf in the running dev container is correctly rebuilt (accept-flake-config = false); the image rebuild was NOT the problem
- **Found:** ~/.local/share/nix/trusted-settings.json contains {extra-substituters: {cache.numtide.com: true}, extra-trusted-public-keys: {niks3...: true}} - a saved 'y' from the old trusted-user era, persisted in the mounted home
- **Found:** Nix's ConfigFile::apply consults the saved trusted-list before accept-flake-config, so a saved 'true' re-enables flake nixConfig even with accept-flake-config = false
- **Found:** The flake nixConfig being applied is what makes `substituters` and `trusted-public-keys` client-overridden; RemoteStore::setOptions forwards only overridden settings, which is why these two specifically are warned about
- **Found:** Trailing-slash bug: nix's built-in client default is https://cache.nixos.org/ but generate_nix_conf emitted https://cache.nixos.org; the daemon matches exactly and only compensates by APPENDING a missing slash, never stripping an extra one -> the default was rejected while numtide (byte-identical) passed silently. That asymmetry explains why only cache.nixos.org was named
- **Found:** The plan's claim that accept-flake-config = false 'deterministically suppresses the interactive prompt path' is WRONG: false means do-not-auto-accept; with an interactive TTY and no saved answer nix still prompts y/n
- **Found:** cargo fmt in the devshell again sweeps import-ordering churn in cli.rs, dev/build.rs and now dev/image.rs (pre-existing, rustfmt version skew); reverted to keep the commit atomic
- **Decided:** Option A implemented as the general form: trusted-substituters now lists BOTH slash spellings of every substituter (not just a one-character fix to the cache.nixos.org default), so user-supplied extras are covered too
- **Decided:** substituters itself left unchanged (canonical, no trailing slash); only the trust list is widened, which is semantically a no-op for daemon behaviour
- **Decided:** Warnings are cosmetic, not functional: the daemon substitutes using its own server-side config, so caches kept working throughout
- **Open:** The stale trusted-settings.json still needs clearing to stop the flake nixConfig from being forwarded; doing so will likely surface an interactive y/n prompt during cast run instead
- **Open:** Decide how to handle the prompt: drop nixConfig from the global flake template (B), have cast seed trusted-settings.json with false values (C), or accept it (D)
- **Open:** Manual QA: restart the nix-daemon container so the new trusted-substituters takes effect, then confirm the cache.nixos.org warning is gone

## [85eb268] Option A: nixConfig block commented out in global flake template

Committed 85eb268. Root cause of the residual noise was that accept-flake-config=false does NOT suppress nix's interactive approval prompt -- it only means "do not auto-accept". Confirmed empirically by the user: nix prompts on every devshell entry, declining without persisting re-prompts forever, accepting swaps the prompt for the daemon warning. The prompt is unanswerable in cast's dev container because the user is non-trusted and the daemon rejects the settings either way. Implemented Option A: the template's nixConfig block is now shipped commented out with instructions to uncomment for standalone/trusted use. TDD: added template_does_not_declare_a_live_nix_config (line-based check that ignores comment lines) as RED, then commented the block. Verified the template still parses with nix-instantiate --parse. Docs, doc-comments and CHANGELOG updated, including the manual remediation for existing users. Suite 232 green, clippy clean.

- **Found:** accept-flake-config = false does not suppress the prompt; with an interactive TTY nix asks on every devshell entry. The plan's premise here was wrong
- **Found:** Answering N without also answering the follow-up persist prompt saves nothing, so nix re-asks indefinitely; answering y writes trusted-settings.json and converts the prompt into a permanent daemon warning
- **Found:** The commented-out block still satisfies the existing numtide-key-drift guard and the cache.numtide.com content assertion, so the trust anchor stays synchronised with DEFAULT_CAST_JSON
- **Found:** cargo fmt swept the same three unrelated files again (cli.rs, dev/build.rs, dev/image.rs); reverted
- **Decided:** Option A over B (accept-flake-config=true) and C (seed trusted-settings.json): remove the cause rather than manage the symptom; accept-flake-config=false still does useful work defending against project flakes
- **Decided:** Rejected the variant of keeping extra-substituters but dropping extra-trusted-public-keys: a flake declaring a cache without its signing key is broken in exactly the standalone environments the block exists to serve
- **Decided:** Guard test checks for a live nixConfig by scanning non-comment lines rather than a plain substring, so the commented block does not trip it
- **Decided:** Existing users are handled by documentation, not by overwriting their flake; scaffold_if_missing semantics are preserved
- **Open:** Manual QA: recreate the nix-daemon container (new trusted-substituters from 6f0d98c), comment out nixConfig in the real ~/.config/cast/nix/flake.nix, delete ~/.local/share/nix/trusted-settings.json, then confirm devshell entry is silent -- no prompt, no warnings
- **Open:** Task-card questions #2 (/nix ro/rw + socket mount) and #3 (sandbox=true) remain unaddressed
- **Open:** Follow-ups still open: allowed-users scoping to numeric host UID, optional empty-cache stderr warning

## [1d9f308] Dropped nixConfig from flake template outright; cast.json is sole cache source

Committed 1d9f308, superseding the commented-out approach of 85eb268 per maintainer direction: the template is a flake for running cast's harnesses inside cast, and use outside the dev container is explicitly not a supported case, so carrying a commented-out block plus its rationale was dead weight. Removed the block and the verbose comment, leaving a two-line note pointing at cast.json. TDD: tightened the guard to a plain !contains("nixConfig") (RED), then stripped the template. Verified with nix-instantiate --parse. Net -74/+33 lines.

- **Found:** Removing the second copy of the numtide key made the cross-file key-drift guard (numtide_key_matches_default_cast_json) moot, so it was deleted along with the copy it guarded
- **Found:** template_defines_expected_shells_and_cache lost its cache assertions and was renamed to template_defines_expected_shells
- **Decided:** The cache and signing key ship in exactly one place: DEFAULT_CAST_JSON, which is where they actually take effect. Single source of truth removes the drift class entirely rather than guarding against it
- **Decided:** Strengthened global_config's contains_numtide_cache from a prefix check to a full-key assertion, since it is now the sole anchor
- **Decided:** Out-of-cast use of the global flake is explicitly not a supported case; docs no longer offer an uncomment path
- **Open:** Manual QA unchanged: recreate the nix-daemon container, delete the nixConfig block from the real ~/.config/cast/nix/flake.nix, remove ~/.local/share/nix/trusted-settings.json, confirm silent devshell entry with cache hits

