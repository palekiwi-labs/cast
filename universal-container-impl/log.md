# Project Log

## [0dbafc9] Design session: universal container

Discussed and designed the universal container feature end-to-end. Consulted Claude Opus for architectural critique. Produced a task card, implementation plan, and feature branch ready for TDD implementation.

- **Found:** The dominant unaddressed risk was volumes not Dockerfiles: a universal image with per-agent-launched mounts means subprocess agents run without their own config and with the wrong cache/local volume — identified by Opus critique
- **Found:** Dockerfile CMD is dead code in the cast run path (build_command always passes an explicit cmd vector); CMD [bash] in universal Dockerfile is purely ergonomic
- **Found:** The COPY --from=node ordering concern for claudecode is a non-issue: both the COPY and npm install stay inside the claudecode fragment, so ordering is preserved intra-fragment
- **Found:** apt package divergence across agents dissolves when using the union set in the shared preamble
- **Found:** Option A (static Dockerfile.dev.universal) was rejected on correctness grounds: it cannot satisfy the subset selection requirement (Decision 2) without 2^N-1 static files
- **Decided:** universal_container: bool config flag (default false) — no CLI changes; cast run opencode still works identically when false
- **Decided:** Agent inclusion is determined by agent_versions map: only agents with a pinned version are included in the universal image
- **Decided:** Image tag uses a SHA-256 content hash of the sorted agent→version pairs (first 12 hex chars), not a concatenated version string; a human-readable Docker --label is added for inspectability
- **Decided:** Universal volumes: {namespace}-universal-cache and {namespace}-universal-local named volumes replace the per-agent cache/local volumes; namespace respects config.volumes_namespace
- **Decided:** All included agents' config dirs (~/.config/opencode, ~/.claude + ~/.claude.json, ~/.pi) are bind-mounted simultaneously — paths are distinct so no collision occurs
- **Decided:** prepare_host() is called for every included agent (not just the launched one) in universal mode
- **Decided:** Dockerfile uses Option B-prime: composed fragments with a union-apt preamble (ca-certificates curl git ssh), per-agent installation fragments, and a shared postamble with CMD [bash]
- **Decided:** New dockerfile_snippet() method added to the Agent trait for fragment extraction
- **Decided:** New config_mount_args() method added to the Agent trait to separate config-dir bind mounts from data volume mounts — enables clean composition in universal mode
- **Decided:** Container naming, port assignment, and build_command logic are unchanged
- **Decided:** Container sharing (reusing a running container for a second agent) is out of scope for this task
- **Open:** Exact SHA-256 crate to use for image tag hashing (sha2 is already in Cargo.toml — confirm before Phase 3)
- **Open:** Whether existing per-agent Dockerfiles should be refactored to reuse the extracted fragments (deduplication) or left as-is — deferred as a separate follow-up to keep this change small

## [3fdf130] Phase 1 complete: universal_container config field

Implemented Phase 1 of the universal container plan (commit 3fdf130). Added the `universal_container: bool` config flag with `#[serde(default)]` (defaults false for backward compatibility) and a `Config::validate()` cross-field invariant helper that rejects enabling universal mode without any pinned agent_versions. Five unit tests cover: deserialise-true, absent-defaults-false, validate-rejects-empty, validate-accepts-with-versions, and validate-accepts-default. All 280 crate tests pass; clippy and rustfmt clean on the changed file.

- **Found:** Config has no #[serde(deny_unknown_fields)], so adding the new field is non-breaking for figment/serde merging
- **Found:** Config has many required (non-default) fields, so unit tests serialise Config::default() to a JSON Value and patch the field under test rather than enumerating every required field
- **Decided:** validate() is a standalone helper on Config, not yet wired into the loader; it will be consumed in Phase 5 (run_agent/build_agent branching) per the plan
- **Decided:** Used struct-literal syntax (Config { universal_container: true, ..Default::default() }) in tests to satisfy clippy's field_assignment_outside_initializer lint
- **Open:** Whether to call Config::validate() from load_config_from() eagerly — deferred to Phase 5 to keep Phase 1 scope minimal and avoid surprising existing callers

## [ea7b7dd] Phase 2 complete: Dockerfile fragment assembly

test

## [ea7b7dd] Phase 2 complete: Dockerfile fragment assembly

Implemented Phase 2 of the universal container plan (commit ea7b7dd). Created the pure-function Dockerfile assembler that composes a shared preamble (union apt deps + Nix config + ARG TARGETARCH) and postamble (git safe.directory + user/dir creation + CMD bash) with per-agent installation fragments loaded via the new Agent::dockerfile_snippet() trait method. assemble() sorts fragments by agent name for deterministic output matching the Phase 3 image-tag hash. Five unit tests cover: opencode fragment + structure invariants, claudecode COPY-before-npm ordering, opencode+pi subset excluding claudecode, all-three inclusion, and input-order independence. All 285 crate tests pass; clippy and rustfmt clean.

- **Found:** Apt dep union across all three agents: ca-certificates, curl, git, ssh (opencode lacks ssh, claudecode lacks curl)
- **Found:** All three existing Dockerfiles share an identical Nix config block and ENV trio (PATH nix, GC_NPROCS=1, NIX_REMOTE=daemon) — moved verbatim to the preamble
- **Found:** clippy prefers .to_vec() over iter().copied().collect() for slice-to-Vec conversion
- **Decided:** Postamble mkdir is a STATIC constant creating the full union of all known agent config dirs (.cache, .claude, .config, .local, .pi) regardless of which subset is included — empty dirs are harmless and keep the postamble simple. No Phase 2 test constrains mkdir contents.
- **Decided:** assemble() sorts agents by name() internally so fragment order is deterministic and matches the sorted agent->version map used for the Phase 3 image-tag hash
- **Decided:** Fragment ARG names use {AGENT}_VERSION (OPENCODE_VERSION, CLAUDECODE_VERSION, PI_VERSION) to match the Phase 3 build-arg convention, replacing the per-Dockerfile AGENT_VERSION
- **Decided:** TARGETARCH is declared once in the preamble; fragments reference ${TARGETARCH} without re-declaring it

## [b2c3850] Phase 3 complete: universal image tag and build function

Implemented Phase 3 of the universal container plan (commit b2c3850). Added the SHA-256 content-hashed image tag (universal_image_tag) and the build orchestrator (ensure_universal_image). The tag serialises sorted agent=version pairs, hashes with SHA-256, and takes the first 12 hex chars — stable for a given config regardless of insertion order. The build function wires dockerfile::assemble with per-agent {AGENT}_VERSION build args + a cast.universal.agents Docker label, mirroring the per-agent ensure_image flow. Six unit tests cover all four plan-specified tag properties plus tag format and build-arg construction. All 291 crate tests pass; clippy and rustfmt clean.

- **Found:** sha2 0.11.0 + digest 0.11.2 already in Cargo.toml/Cargo.lock — open question resolved
- **Found:** config/approval.rs already uses sha2 with the hex crate, providing the canonical usage pattern to mirror
- **Found:** build_docker_build_args does not support --label and puts context_path as the final positional arg; --label must be inserted before it, justifying a separate helper rather than reuse
- **Found:** ensure_image (the per-agent equivalent) has no unit tests — it is pure I/O wiring; the testable logic (tag hash, build-arg naming) lives in extracted pure functions
- **Decided:** Resolved open question from design session: sha2 = 0.11 is already in Cargo.toml — confirmed via Cargo.lock (sha2 0.11.0, digest 0.11.2)
- **Decided:** Mirrored the existing sha2 usage pattern from config/approval.rs (Sha256::new + update + hex::encode(finalize)) instead of the one-shot digest() API for consistency
- **Decided:** Made IMAGE_BASE and CAST_VERSION pub(crate) in dev::image.rs to avoid duplicating the constants in the universal module
- **Decided:** build_docker_build_args is NOT modified (per plan). Instead wrote a self-contained build_universal_docker_args helper that takes pre-built arg tuples (5 params, clippy-clean) and adds --label support. The per-agent {AGENT}_VERSION naming is constructed by the caller (ensure_universal_image).
- **Decided:** ensure_universal_image derives the tag from the FULL config.agent_versions map (not just the agents passed in agents_with_versions) so the tag is stable for a given config regardless of which subset triggered the build
- **Decided:** Added a unit test for build_universal_docker_args beyond the plan's 4 tag tests — it verifies per-agent arg naming, label format, no_cache flag, and context-path-as-last-arg ordering

## [f2312b8] Phase 4b complete: config_mount_args trait extraction

Refactored each agent's config-directory bind mount logic from extra_run_args into a dedicated config_mount_args trait method on Agent (commit f2312b8). Each agent's extra_run_args now calls self.config_mount_args() internally, preserving identical non-universal behavior. This extraction enables Phase 4c to compose the union of all config mounts in universal mode. All 262 existing tests pass; clippy clean. Pre-existing rustfmt issues in unrelated files (cli.rs, schema.rs, nix_daemon) were noted but left untouched as out of scope.

- **Decided:** config_mount_args signature: fn config_mount_args(&self, _config: &Config, opts: &RunOpts) -> Result<Vec<String>> — config param kept for signature consistency/future use but unused in current impls (prefixed _config where unused)
- **Decided:** Flake mount logic stays in extra_run_args for non-universal mode and will be inlined separately in build_universal_run_args (not extracted into config_mount_args — it is not a config-dir mount)
- **Decided:** opencode workspace-conflict guard (skip mount if config dir == workspace root) moved into config_mount_args to preserve behavior

## [653ef12] Phase 4 complete: universal volume and mount strategy

Implemented Phase 4a+4c of the universal container plan (commit 653ef12). Created dev/universal/volumes.rs with two public functions: build_universal_data_volume_args (shared {ns}-universal-{cache,local} named volumes) and build_universal_run_args (composes 4 layers: universal data volumes, union of config_mount_args from all included agents, env passthrough from launched agent only, user flake mount if present). Added env_passthrough_args trait method to Agent (mirroring the config_mount_args pattern) to enable env passthrough composition via the trait; each agent's extra_run_args now calls it internally (behavior-preserving). Seven unit tests cover all plan-specified properties. All 269 crate tests pass; clippy and rustfmt clean.

- **Found:** All three agents' config mount target paths are naturally distinct (~/.config/opencode, ~/.claude, ~/.claude.json, ~/.pi) — no collision occurs when all are included simultaneously, confirming the design session decision
- **Found:** Pre-existing rustfmt issues in cli.rs, schema.rs, and nix_daemon module are present on the base commit and out of scope for this phase
- **Decided:** Added env_passthrough_args trait method (not in original plan text) because build_universal_run_args needs to call the launched agent's env passthrough function via the trait — without it, there is no way to dispatch to the correct agent's env::build_passthrough_env_args through &dyn Agent
- **Decided:** Flake mount logic is inlined in build_universal_run_args rather than extracted into a shared helper — avoids scope creep; the duplication across extra_run_args impls is left as a future refactor opportunity
- **Decided:** Env passthrough is from the LAUNCHED agent only (not all included agents) — subprocess agents inherit the container env, so including their passthrough vars would be redundant or conflicting
- **Decided:** The no_two_mounts test extracts container target paths from -v args using splitn(3, ':') and verifies uniqueness via a HashSet — this is the critical safety property ensuring no bind-mount collisions when multiple agents' config dirs are composed

## [73678c9] Phase 5 complete: run_agent and build_agent universal branches

Wired the universal container branching into run_agent and build_agent (commits 130202a, 183a754, 73678c9). Three incremental slices: (1) agent registry + validation helpers, (2) dispatch_run refactor, (3) the actual branching. When config.universal_container is true, run_agent validates the requested agent is pinned, resolves all included agents, ensures the universal image, calls prepare_host for ALL included agents, builds universal mounts, and dispatches via the shared helper. build_agent builds the combined image directly. The non-universal paths are completely unchanged. All 277 crate tests pass; clippy and rustfmt clean.

- **Found:** run_in_container is only called from run_agent and exec (shell.rs uses docker exec into a running container, not run_in_container)
- **Found:** The agents are static unit structs (OpenCode, ClaudeCode, Pi) with no existing registry — created all_agents() as the single place to register a new agent harness for universal-mode resolution
- **Found:** validate_agent_included error message names the missing agent and mentions agent_versions for actionable guidance
- **Found:** resolve_included_agents silently ignores unknown agent names in agent_versions (defensive: unknown keys don't cause errors, they're just not included)
- **Decided:** Created dev::universal::registry with three pure functions: all_agents() (static registry), resolve_included_agents(config) (maps agent_versions to Agent+version pairs borrowing into config), and validate_agent_included(config, name) (bails with actionable message)
- **Decided:** Extracted dispatch_run as a private helper from run_in_container — assembles flags, dispatches on tty_mode, logs duration. Both run_in_container (non-universal) and the universal branch in run_agent call it, avoiding code duplication.
- **Decided:** Did NOT create a run_in_container_universal function — it would need 8 args (clippy too_many_arguments). Instead, the universal branch in run_agent builds flags inline (build_docker_run_flags + build_universal_run_args) and calls dispatch_run directly. run_agent is in the same module so it can call the private helper.
- **Decided:** Config::validate() is now consumed at runtime in the universal branches of both run_agent and build_agent, catching the 'universal without agent_versions' invariant at the earliest point. NOT wired into load_config_from() to avoid surprising non-run/build callers (cast config show, cast port, etc.).
- **Decided:** build_agent in universal mode prints the combined image tag via eprintln for user feedback, mirroring the per-agent path's user-facing output
- **Decided:** Universal branch in run_agent is an early return (if config.universal_container { ... return dispatch_run(...) }) so the non-universal path is completely unchanged below it
- **Open:** cast exec and cast shell do NOT branch on universal_container — they use the per-agent image path. In universal mode, no per-agent image exists, so these commands would fail. Extending universal mode to exec/shell is a follow-up (out of scope per plan).
- **Open:** No end-to-end test exists for the universal run path (requires real Docker). The testable pure-function logic is fully covered: validate_agent_included (3 tests), resolve_included_agents (5 tests), universal_image_tag (5 tests from Phase 3), build_universal_run_args (7 tests from Phase 4). The I/O wiring follows the same untested pattern as the existing non-universal run_agent/build_agent.
- **Open:** Phase 6 (documentation) is the remaining phase

