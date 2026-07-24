# Project Log

## [0dbafc9] Merged universal-container + select-global-shell into nix-native-harnesses; spec + plan drafted

Consolidated three suspended/related tasks into one unified effort after a design discussion in the palekiwi control center (driven by the cast-agent-design task, which needs multi-harness containers as its runtime substrate). Created a unified task, spec (root), and master plan (root) under the new nix-native-harnesses context, plus a deferred follow-up task. Closed the three superseded task cards. Work continues on the existing feat/universal-container branch.

- **Found:** feat/universal-container is fully implemented (8 commits, ~45 tests); ~two-thirds is provisioning-agnostic and reusable (config_mount_args agent.rs:88, build_universal_run_args volumes.rs:43, shared volumes volumes.rs:17, prepare_host loop run.rs:219, all_agents registry.rs:16)
- **Found:** Global-flake wrapping already exists at dev/build_command.rs:24-33 (nix develop <global-flake> -c ...); shell selection is just appending #<name>
- **Found:** Top-level Config has no deny_unknown_fields (schema.rs), so dropping agent_versions is a silent no-op on load — no deprecation plumbing needed
- **Found:** Daemon image tag is version-coupled (nix_daemon/image.rs:13) so 0.2.0 rebuilds it, but nixos/nix:2.34.6 stays pinned so the persistent unversioned /nix volume keeps valid paths — no breakage from the bump alone
- **Found:** accept-flake-config is a client-side (dev container) gate; trusted-users=root* on the daemon (nix_daemon/config.rs:14) is the already-satisfied second gate
- **Decided:** universal_container boolean collapses into global-shell selection; universal mounts/volumes become the unconditional default
- **Decided:** flake.lock is the version source of truth; drop agent_versions (silent ignore + release note); kill content-hash tag; image name becomes plain localhost/cast:0.2.0
- **Decided:** Auto-scaffold the global flake from an embedded template on missing; also expose a nix templates output; keep cast init
- **Decided:** accept-flake-config = true goes in the dev container system /etc/nix/nix.conf as a hard prerequisite (verify empirically in the plan)
- **Decided:** Bump to 0.2.0, tied to branch merge
- **Decided:** nix-daemon/ /nix volume version-skew handled separately (nix-daemon-volume-version-skew.md); keep nixos/nix:2.34.6 pinned
- **Open:** Exact name of the shell-selection config field and precedence vs existing use_flake_path
- **Open:** Whether the universal shell is named universal/agents/all
- **Open:** Empirical confirmation that accept-flake-config alone suffices for non-interactive numtide fetches (planned verification step)

## [d1e0774] Phase 1 GREEN: global_shell config + build_command agent-name wiring

Added global_shell: Option<String> to Config and wired agent_name into build_command so the global flake reference includes a #<shell> fragment. All 281+ tests pass.

- **Found:** build_command already had agent in scope via agent.name() on the Agent trait, making the wiring clean
- **Found:** exec.rs build_exec_cmd needed a new agent_name param since it builds commands without an agent object at the call site
- **Found:** shell.rs has access to agent.name() so the wiring is natural there too
- **Found:** Removing agent_versions/universal_container from Config now would break ~60 references across run.rs, build.rs, registry.rs, image.rs, and per-agent mod.rs files — deferred to Phase 2 as planned
- **Decided:** Keep agent_versions and universal_container fields in Config for Phase 1; mark them deprecated in doc comments. Phase 2 removes them along with the universal_container branch code.
- **Decided:** Use agent.name() (not base_command()) as the shell name, since name() is the agent identifier (e.g. 'opencode') which matches the expected devShell name convention
- **Open:** Phase 2: delete universal_container branch in run.rs/build.rs, route single path through build_universal_run_args + all_agents() prepare_host loop
- **Open:** Phase 3: strip Docker-image assembly (dockerfile_snippet, universal_image_tag, ensure_universal_image)

## [351737b] Phase 2 GREEN: universal mounts + prepare_host loop unconditional

- **Found:** universal/image.rs is now dead code (ensure_universal_image, universal_image_tag, dockerfile::assemble) — nothing imports it; deferred to Phase 3
- **Found:** config_test.rs and loader.rs tests also referenced agent_versions — both cleaned up
- **Found:** 312 lines deleted; 30 added — largest deletion was registry.rs losing resolve_included_agents/validate_agent_included and all their tests
- **Decided:** Stub universal/image.rs agent_versions references with BTreeMap::new() rather than deleting the file now — keeps Phase 2 atomic; Phase 3 deletes the whole file
- **Decided:** Per-agent resolve_version now always resolves 'latest'; version pinning moves to the global flake.lock as per spec
- **Open:** Phase 3: delete universal/image.rs, universal/dockerfile.rs, Dockerfile.frag.*, agent.rs::dockerfile_snippet(); strip per-agent harness installs from Dockerfiles; add accept-flake-config=true to dev container nix.conf; rename image to localhost/cast:{version}

## [18c54e2] Phase 3 GREEN: single harness-free dev image

Collapsed the three per-agent Docker images into one shared, harness-free dev image built from assets/Dockerfile.dev. Harnesses are now provided solely by the global Nix devShell. All 224+ tests pass; clippy/fmt clean across the workspace (all features).

- **Found:** universal/image.rs and universal/dockerfile.rs were already fully self-contained dead code (no external importers) after Phase 2 — deletion was clean
- **Found:** dev/version/ module was reachable only via the three agents' resolve_version; once harness installs left the Dockerfile it became entirely dead
- **Found:** Rust does not warn on unused function parameters, so build_agent can keep its &dyn Agent param (_agent) to preserve the per-agent `cast build <agent>` CLI surface unchanged
- **Found:** config.version_cache_ttl_hours remains in the schema as an unused pub field (no dead_code warning) — deferred removal to avoid schema-test churn
- **Decided:** Remove version-resolution machinery (dev/version/, resolve_version, GitHub/npm fetchers) in Phase 3 even though the master plan's Dies list did not enumerate it — its only purpose was picking a harness version for Docker installs, so it is dead once installs are gone and contradicts 'versions pinned by flake.lock'
- **Decided:** image_tag() takes no args and returns plain localhost/cast:{cast_version}; all agents resolve to the same tag and share one image
- **Decided:** accept-flake-config=true written into the dev container /etc/nix/nix.conf alongside experimental-features via a single printf
- **Decided:** Union mkdir keeps .cache/.claude/.pi/.config/.local so one image serves every harness
- **Open:** Phase 4: embed global-flake template + auto-scaffold on missing ~/.config/cast/nix/flake.nix
- **Open:** Phase 5: bump Cargo.toml to 0.2.0 (re-tags daemon image); consider removing now-unused version_cache_ttl_hours config field
- **Open:** Phase 6: docs for the nix-native model
- **Open:** Empirical verification still pending: confirm accept-flake-config alone suffices for non-interactive cache.numtide.com fetches

## [a818b23] Phase 4 GREEN: embedded global-flake template + auto-scaffold

Shipped the global-flake template as an asset directory and wired auto-scaffold into resolve_run_opts. Also exposed templates.global/templates.default from cast's flake, sourced from the same asset dir. 227 tests pass; clippy/fmt clean; nix flake show confirms both template outputs evaluate.

- **Found:** nix flake template `path` must be a directory containing flake.nix, so the embedded asset lives at assets/global-flake-template/flake.nix and include_str! points at it — single source of truth for both the binary and `nix flake init -t`
- **Found:** resolve_run_opts already resolves host_home_dir and checks flake presence, so scaffolding slots in cleanly right before that check (scaffold then detect => present on first run)
- **Found:** templates are system-independent so they go outside flake-utils.eachDefaultSystem, merged via the `{ ... } // eachDefaultSystem(...)` pattern
- **Decided:** Scaffold failures are soft (warn + continue), not fatal: a missing global flake is degraded UX, not a hard error, matching the host_name soft-fail precedent
- **Decided:** scaffold_if_missing takes an explicit home_dir Path (not dirs::home_dir internally) so it is testable under nix-build sandbox constraints via TempDir
- **Decided:** Template uses a baseInputs + mkShell composition reference pattern with per-agent shells and a `universal` shell (spec's open question resolved to `universal`)
- **Open:** Phase 5: bump Cargo.toml to 0.2.0 (re-tags daemon image); consider removing unused version_cache_ttl_hours field
- **Open:** Phase 6: docs for the nix-native model
- **Open:** Empirical: verify accept-flake-config alone suffices for non-interactive cache.numtide.com fetches; verify llm-agents.nix attr names (opencode/pi/claude-code) match the real flake before relying on the template end-to-end

## [8cce186] Phase 5 GREEN: bump to 0.2.0, drop version_cache_ttl_hours

Bumped cast crate to 0.2.0 in Cargo.toml, Cargo.lock, and flake.nix common.version. Removed the dead version_cache_ttl_hours config field (declaration + Default). Added a 0.2.0 CHANGELOG entry documenting the nix-native pivot. 227 tests pass; clippy/fmt clean.

- **Found:** version_cache_ttl_hours was referenced only in schema.rs (field + Default) — no tests, docs, or JSON schema depended on it, so removal was clean with zero test churn
- **Found:** Version is env!(CARGO_PKG_VERSION)-driven for both dev image (dev/image.rs) and nix-daemon image (nix_daemon/image.rs), so the single Cargo.toml bump re-tags both automatically
- **Found:** flake.nix hardcodes version separately (common.version) and needed a manual bump to 0.2.0
- **Found:** A separate master task tag-0-1-0-release.md tracks tagging 0.1.0 before this branch merges — a human/release concern, left untouched
- **Decided:** Removed version_cache_ttl_hours now rather than deferring further: top-level Config has no deny_unknown_fields, so any leftover key in a user's cast.json is silently ignored — no migration needed
- **Decided:** CHANGELOG structured as Changed/Added/Removed per Keep a Changelog, capturing the full nix-native pivot (harness-free image, global_shell, template/scaffold, accept-flake-config, and the three removed config fields)
- **Open:** Phase 6: docs for the nix-native model (docs/config/*, docs/agents.md) — global-shell selection, template/auto-scaffold flow, accept-flake-config rationale
- **Open:** Empirical verification still pending: accept-flake-config sufficiency for non-interactive numtide fetches; confirm llm-agents.nix attr names before relying on the template end-to-end

## [9224622] Phase 6 complete: docs for the nix-native model

Updated the cast crate docs (plus top-level quick-start) to describe the nix-native harness model shipped in phases 1-5. Docs-only change; all cast tests still pass. This completes the final phase of the nix-native-harnesses master plan.

- **Found:** docs had no stale references to removed internals beyond the two intentional mentions (agents.md noting the trait no longer resolves versions; reference.md noting agent_versions is silently ignored)
- **Found:** quick-start.md previously claimed a per-agent 'opencode sandbox image' build — corrected to the shared harness-free image plus flake scaffold + devShell entry
- **Decided:** Added a new 'Nix-native harness provisioning' section to agents.md and a harness-provisioning mode (0) to nix/overview.md, keeping flake-integration.md as the single deep-dive for shell selection, auto-scaffold, template shape, and accept-flake-config/numtide rationale
- **Decided:** Kept the Agent trait lifecycle description but rewrote it to the 4 program-specific responsibilities that remain (config mounts, prepare_host, run args, command wrapping) since version resolution and image building are gone
- **Decided:** Reframed config reference: global_shell documented under Nix Settings; agent_versions section retained as a deprecation note rather than deleted, to answer users who still have the key
- **Open:** Empirical verification still pending (not doc work): confirm accept-flake-config alone suffices for non-interactive cache.numtide.com fetches; confirm llm-agents.nix attr names (opencode/pi/claude-code) end-to-end; clean-host run scaffolds -> shell -> binary resolves
- **Open:** All six phases now GREEN; remaining task-completion gate is the empirical/e2e verification and human QA before marking the task complete and merging the branch

## [04836fd] Dropped 'universal' segment from data volume names

Renamed the shared cache/local named volumes from {namespace}-universal-{cache,local} to {namespace}-{cache,local}. Confined to dev/universal/volumes.rs (format strings, doc comments, 4 test assertions). All cast tests pass; clippy/fmt clean. Committed as 04836fd.

- **Found:** The 'universal' volume segment existed only to contrast with pre-pivot per-agent data volumes; dead terminology now that the union path is the sole mode
- **Found:** No docs referenced the specific volume names (only volumes_namespace), so the rename was confined to volumes.rs
- **Decided:** Keep function/module names (build_universal_data_volume_args, universal/ module) unchanged; the 'universal' name now refers to the union-container concept, not volume naming
- **Decided:** Pre-release, so orphaned cast-universal-* volumes on dev machines are acceptable (regenerable cache/local); no migration needed

## [cb03c21] Fixed numtide cache rejection: daemon must trust the numtide key

Empirical QA (trace nix-native-harnesses/.../cast-shell.txt) showed harness substitutes from cache.numtide.com were rejected as unsigned and rebuilt from source. Root cause: signature verification runs in the nix-daemon container, whose trusted-public-keys (generate_nix_conf) only included cache.nixos.org. accept-flake-config made the client USE the flake-declared substituter, but the flake's extra-trusted-public-keys applies client-side and does not propagate into the daemon's trust set. Fix: added cache.numtide.com + key to the daemon's base substituters and trusted-public-keys, mirroring cache.nixos.org. Committed cb03c21; all cast tests green, clippy/fmt clean.

- **Found:** Signature verification is performed by the nix-daemon container, not the dev-container client; the daemon uses its OWN trusted-public-keys
- **Found:** accept-flake-config forwards the flake's extra-substituters (numtide WAS contacted) but the flake's extra-trusted-public-keys does NOT reach the daemon's trust set
- **Found:** This disproves the earlier log assumption that 'trusted-users = root *' was the sufficient second gate — the real gate is the daemon trusting the numtide signing KEY
- **Decided:** Bake cache.numtide.com substituter + public key into the daemon's default generate_nix_conf unconditionally, mirroring cache.nixos.org, since the nix-native model depends on numtide harnesses for every agent
- **Decided:** Prefer daemon-side defaults over relying on flake nixConfig forwarding, making harness fetches work out of the box regardless of the user's global flake
- **Open:** Requires rebuilding/restarting the nix-daemon container so the new NIX_CONFIG takes effect; paths already built from source stay but future fetches hit the cache
- **Open:** Re-run the empirical QA (cast run/shell -> cache HIT, no rebuild) to confirm the fix end-to-end

## [f33c89b] Root cause: wrong numtide cache key name; reverted cb03c21 + investigation edits

Empirical QA showed harness substitutes from cache.numtide.com were rejected as unsigned and rebuilt from source. A multi-step investigation into client-side vs daemon-side trust propagation (Dockerfile trusted-users, daemon config keys, accept-flake-config) was chasing the wrong problem. The actual cause: the numtide binary-cache public key we carried everywhere (cb03c21 daemon default, global flake template, user cast.json) had the wrong key NAME and value: cache.numtide.com-1:2ps1... nix matches trusted keys by name, and numtide signs paths with niks3.numtide.com-1 (confirmed via the narinfo Sig line for pi-0.82.0 and numtide's published README key). So the entry was never a candidate. palekiwi.cachix.org worked because its name+value were correct. Fix = one-line key correction in the global flake template; the flake's nixConfig.extra-trusted-public-keys DOES reach the daemon, so no baked key, no trusted-users change, no daemon default needed. cb03c21 and all investigation edits reverted.

- **Found:** numtide signs with key name niks3.numtide.com-1 (from narinfo Sig: on /nix/store/...-pi-0.82.0 and numtide llm-agents.nix README); correct key is niks3.numtide.com-1:DTx8wZduET09hRmMtKdQDxNNthLQETkc/yaX7M4qK0g=
- **Found:** The wrong key cache.numtide.com-1:2ps1kLBUWjxIneOy1Ik6cQjb41X0iXVXeHigGmycPPE= was in cb03c21, the flake template, and user cast.json — all originating the same typo/wrong value
- **Found:** nix matches trusted-public-keys by key name first; a wrong-named key is silently ignored, producing 'not signed by any of the keys in trusted-public-keys' even when the substituter is contacted
- **Found:** The flake's nixConfig.extra-trusted-public-keys IS honoured for daemon substitution once the key is correct — the earlier client/daemon-trust theories were unnecessary
- **Found:** Probe 2 (client --option trusted-public-keys still rejected) did NOT prove daemon-side-only; it rejected because the key value was wrong regardless of location
- **Decided:** The single real fix is correcting the key in assets/global-flake-template/flake.nix (committed f33c89b)
- **Decided:** Reverted cb03c21 entirely (dropped via git reset --mixed to parent 04836fd) — daemon config back to cache.nixos.org only; no numtide baked in the daemon
- **Decided:** No Dockerfile.dev trusted-users change and no numtide in cast.json are needed; the flake nixConfig alone suffices
- **Decided:** No binary-cache key is baked anywhere in cast source — numtide trust lives only in the user-editable/scaffolded global flake
- **Open:** Empirically re-confirm on a fully clean host: scaffold flake with corrected key -> cast run/shell -> numtide cache HIT, no rebuild (user confirmed HIT interactively this session)
- **Open:** Consider a guard test asserting the template contains niks3.numtide.com-1 to prevent regression of the key value

## [6950cc1] Fixed stale 'ocx' command name in error messages

User reported `cast shell pi` printed "Run 'ocx run pi' to start it" — a stale reference to the pre-rename binary name. Corrected two user-facing 'not running' messages to use 'cast'. Committed 6950cc1.

- **Found:** shell.rs:28 and daemon.rs:121 both still said 'ocx' (pre-rename binary name); CLI binary is 'cast' per cli.rs command name
- **Found:** No tests asserted these message strings, so it was a safe string-only correction
- **Found:** port.rs:87 also contains 'ocx-rs is awesome' but it is inert test-hash input data, not user-facing — left as-is
- **Decided:** Fixed both messages ('cast run <agent>' and 'cast nix-daemon start') in one atomic fix commit
- **Decided:** Left the port.rs test-data 'ocx' string untouched since changing it would alter a hash-input fixture without user benefit
- **Open:** Consider a wider grep for any remaining 'ocx' references in docs/assets if the rename was incomplete elsewhere

## [6950cc1] Create v0.1.0 release tag on master

Addressed task tag-0-1-0-release: created an annotated git tag for the 0.1.0 release on master HEAD before feat/universal-container merges (which bumps to 0.2.0).

- **Found:** Both crates (cast, cast-mcp-client) are at version 0.1.0 in Cargo.toml on master
- **Found:** No tags existed in the repo prior to this
- **Found:** Master HEAD is 0dbafc9 (merge #58 feat/cast-host-name)
- **Found:** No GPG signing configured
- **Decided:** Created annotated tag v0.1.0 (with conventional 'v' prefix) pointing at 0dbafc9
- **Decided:** Did not push per no-push protocol; push left to human
- **Open:** Confirm tag naming convention: v0.1.0 vs bare 0.1.0
- **Open:** Push v0.1.0 to origin
- **Open:** Mark task complete after confirmation

## [910e066] Phase 7 item 1: converged cast exec onto universal mount topology

Fixed review should-fix #1. cast exec now shares the exact mount topology as cast run: shared {ns}-cache/{ns}-local data volumes + the union of all agents' config mounts. Committed 910e066; 224 lib tests pass, clippy/fmt clean.

- **Found:** Agent::extra_run_args and all three per-agent build_data_volume_args were only reachable via run_in_container (exec's path); once run_in_container switched to build_universal_run_args they became fully dead code
- **Found:** run_agent already inlined the universal path; delegating it to run_in_container removed the duplicated dispatch/prepare_host/args logic entirely
- **Decided:** Introduced a thin build_session_run_args(launched_agent, config, opts, env) helper wrapping build_universal_run_args(all_agents(), ...) as the single seam both run and exec use, with a run.rs test asserting shared volumes + union config mounts + absence of per-agent volumes
- **Decided:** run_in_container now preps host for every all_agents() member (universal mounts) rather than the single launched agent
- **Decided:** Deleted extra_run_args from the Agent trait and opencode/pi/claudecode; migrated the still-meaningful coverage (config_mount_args workspace-conflict, config mounts, env passthrough) to direct method tests
- **Open:** Item 2: hoist scaffold_if_missing out of resolve_run_opts into run_agent to stop unit tests writing to real $HOME
- **Open:** Nits 4-6 remaining (flake.nix comment path, image.rs pub(crate), Config::validate no-op)

## [5a1cea9] Phase 7 complete: scaffold hoist + diff-review nits

Completed the remaining Phase 7 diff-review fixes on feat/universal-container. Item 2 (scaffold hoist) landed as f7e8d39; nits 4-6 + docs note as 5a1cea9. 253 cast tests pass; clippy/fmt clean. Item 1 was already done in a prior session (910e066); Item 3 was resolved incidentally during Item 1.

- **Found:** resolve_run_opts previously called scaffold_if_missing(dirs::home_dir()), so unit tests hitting it wrote the global flake to the real $HOME — the exact nix-sandbox / side-effect violation the review flagged
- **Found:** Config::validate() was a pure no-op never wired into load_config; only its own test referenced it, and dropping it left anyhow::Result unused in schema.rs
- **Found:** IMAGE_BASE/CAST_VERSION had no out-of-module users, so pub(crate) was unwarranted
- **Found:** Item 3's stale doc link was already fixed when Item 1 rewrote the dispatch_run/run_in_container comment
- **Decided:** Introduced scaffold_global_flake() as a dedicated pub helper called by BOTH run_agent and exec before resolve_run_opts, rather than run_agent only — preserves first-run auto-scaffold for exec too while keeping resolve_run_opts pure detection
- **Decided:** Regression test overrides HOME to a TempDir (save/restore via unsafe set_var, edition 2024) and asserts no flake is written — the 'inject a temp home' option from the plan
- **Decided:** Dropped Config::validate() now (Item 6 default recommendation) instead of wiring it into load_config, since no real invariant exists yet
- **Decided:** Split into two commits: Item 2 (behavioural, TDD) and nits 4-6 + docs (mechanical)
- **Open:** All Phase 7 items now GREEN; the only remaining task-completion gate is empirical/e2e QA (clean-host scaffold -> shell -> numtide cache HIT, confirmed interactively earlier) and human attestation before marking nix-native-harnesses complete and merging
- **Open:** Consider a guard test asserting the template carries the correct niks3.numtide.com-1 key value (carried over from earlier open item)

