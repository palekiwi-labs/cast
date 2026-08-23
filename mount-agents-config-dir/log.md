# Project Log

## [f5b2d32] Added universal .agents config_dir helper

Committed the foundation module crates/cast/src/dev/universal/config_dir.rs mirroring the pi/config_dir.rs pattern but for the cross-harness .agents directory. Provides get_config_dir(base) -> base/.agents and ensure_config_dir(base) which create_dir_all the host dir so Docker bind mounts don't auto-create root-owned paths. Registered as pub mod config_dir in universal/mod.rs. Two unit tests pass (GREEN).

- **Decided:** .agents belongs in the universal layer (crates/cast/src/dev/universal/), not tied to any single agent, because it is the vendor-neutral cross-harness standard read by OpenCode and Pi natively.

## [cd470a8] Mounted .agents into universal run args + host prep wiring

Committed the .agents bind mount in build_universal_run_args (volumes.rs) as a universal step alongside the shared -cache/-local data volumes, plus a dedicated build_agents_config_args helper. Added universal::prepare_host in universal/mod.rs which calls config_dir::ensure_config_dir(home) and wired it into run_in_container next to the agent prepare_host loop. All 234 cast lib tests pass; the existing no_two_mounts_share_the_same_container_target test confirms no duplicate .agents target.

- **Decided:** Placed .agents in the universal mount layer (step 1 of build_universal_run_args), not in any agent's config_mount_args, because it must be present for every session regardless of included agents.
- **Decided:** Host prep is a single universal::prepare_host call rather than per-agent, since .agents is not an agent-specific concern.

## [fb342f2] mount-agents-config-dir implementation complete

Implementation complete on branch feat/mount-agents-config-dir (3 commits). The cross-harness .agents directory is now mounted by default in cast sandboxes:

1. f5b2d32 — universal/config_dir.rs helper (get_config_dir/ensure_config_dir) for ~/.agents.
2. cd470a8 — .agents bind mount added to build_universal_run_args (universal layer, present for every session regardless of included agents); universal::prepare_host wired into run_in_container.
3. fb342f2 — Dockerfile.dev pre-creates and chowns /home/${USERNAME}/.agents; image.rs test guards it.

Validation: cargo test -p cast --lib -> 235 passed (+4 new). cargo clippy --all-targets -D warnings -> clean. cargo fmt: zero new diffs (pre-existing drift logged in todo, out of scope). Master task card updated with branch ref and Evidence for both acceptance criteria (test-based, not human-attested).

- **Found:** build_universal_run_args already had a no-duplicate-mount-target test which now also guards the .agents mount.
- **Found:** Dockerfile.dev lives at crates/cast/assets/Dockerfile.dev (include_str), not assets/Dockerfile.dev as the task body suggested.
- **Decided:** Designed .agents as a universal mount (build_universal_run_args step 1) rather than per-agent config_mount_args, because it is the cross-harness standard and must be present for every session.
- **Decided:** Host prep is a single universal::prepare_host call rather than folded into an agent's prepare_host.
- **Decided:** Left pre-existing cargo fmt drift (6 diffs on master) untouched; logged as a low-priority todo to keep this PR atomic.

## [b8eafcc] Cleaned format-on-write noise from run.rs; minimal net diff restored

Before requesting review I discovered commit cd470a8 had bundled unintended reformatting of run.rs (import alphabetization + two CAST_MCP_URL assert unwraps) with the prepare_host wiring. Root cause: the opencode edit/write tool path runs a formatter-on-write that canonicalizes the ENTIRE file on every write. Crucially this formatter DISAGREES with `cargo fmt --check` on the asserts (on-write formatter unwraps ~103-char lines to single line; cargo fmt wants them wrapped). Bash-level writes (git restore, perl -i) do NOT trigger format-on-write, so they preserve master's exact bytes.

Fix applied: restored run.rs from master via `git restore`, then re-applied ONLY the 3-line prepare_host insertion via `perl -0777 -i` (bash, no format-on-write). Committed as corrective commit b8eafcc. Net branch diff for run.rs is now exclusively the prepare_host wiring. All 235 tests still pass.

Takeaway for future edits to files with pre-existing fmt drift: prefer bash-level edits (perl/sed) when a clean minimal diff matters, because the edit tool's format-on-write will re-canonicalize unrelated lines.

- **Found:** The opencode edit/write tools format-on-write, canonicalizing the whole file. This formatter disagrees with cargo fmt on assert wrapping.
- **Found:** Bash-level file writes (git restore, perl -i, sed) bypass the format-on-write path and preserve exact bytes.
- **Decided:** Corrective commit (not amend, per git-commit skill) to scope run.rs back to the intended change.
- **Decided:** Use perl via bash to apply the prepare_host insertion so the format-on-write path does not re-canonicalize the file.

## [b8eafcc] Independent verification of opus review confirms all findings

consultant-gemini-pro independently verified all 12 findings from the opus review against the actual code. Result: every finding VERIFIED (F6 verified-with-nuance — the inert-Dockerfile point is factually correct but benign, not a problem). No new issues found; the opus review was exceptionally rigorous.

Agreed triage:
- FIX NOW (required): F1 (workspace-collision guard).
- FIX NOW (cheap, bundle): F2 (prepare_host tests + exact mount assert), F3 (exact Dockerfile assert), F4 (move test under correct banner), F5 (run.rs doc comments).
- DEFER as follow-up todos: F7 (extract dev::utils ensure_dir/host_home), F10 (docs concepts.md host-access note), F11 (rw vs ro / opt-out — record as design decision, matches .claude/.pi precedent; ro would break in-sandbox skill installs).
- REJECT: F6 (Dockerfile mkdir/chown is cosmetic — prepare_host is the real guard), F8 (name clear enough in context), F9 (both tempfile APIs valid), F12 (irrelevant under squash merge).

- **Decided:** Act on F1-F5 in this branch; defer F7/F10/F11 as logged todos; take no action on F6/F8/F9/F12.
- **Decided:** F11 (rw global instruction dir): defer to a design decision record rather than block the feature — consistent with existing .claude/.pi precedent; ro would break legitimate in-sandbox skill installs.

## [0355b23] [0355b23] F1: workspace-collision guard for .agents mount

Implemented the required review fix F1 (workspace-collision guard) in build_agents_config_args (volumes.rs). TDD vertical slice: RED test asserted the mount is skipped when workspace.root == ~/.agents (failed: mount was produced); GREEN added a guard returning Ok(vec![]) mirroring opencode/mod.rs:40-44. All 236 lib tests pass; clippy clean. Committed as 0355b23 (fix). This is the only behavior change among the review fixes; F2-F5 are test coverage, test tightening, test relocation, and doc-comment refresh.

- **Decided:** Mirror the opencode config_mount_args collision guard pattern (agents_host_dir == opts.workspace.root → Ok(vec![])) rather than a custom sentinel, for consistency with the existing agent guards.
- **Decided:** Commit F1 alone (fix type) since it is the sole behavior change and the only required finding; keep F2-F5 as separate logical commits.

## [e7fbe87] [e7fbe87] F2: prepare_host coverage + exact .agents mount spec

Added F2 test coverage as commit e7fbe87 (test type, no behavior change). F2a: two tests in universal/mod.rs pin prepare_host — it creates ~/.agents under a temp host home and errors when host_home_dir is None. F2b: two direct unit tests in volumes.rs for build_agents_config_args assert the exact mount spec and the None-host-home error path; the composition test's loose contains(\"/.agents:rw\") assertion was tightened to the exact \"/home/alice/.agents:/home/alice/.agents:rw\" string, matching the sibling data-volume test style. All 240 lib tests pass (+4 new); clippy clean. These are characterization tests pinning existing contracts."

- **Decided:** Tighten the composition test to the exact spec string (contains(&"...")) rather than a loose contains("/.agents:rw"), matching the sibling data-volume assertion style and making drift detectable.
- **Decided:** F2 is a pure test commit (no behavior change) — characterization tests pinning the existing prepare_host and build_agents_config_args contracts.

## [1b474b4] [1b474b4] F3/F4/F5: tighten assertions, relocate test, refresh docs

Committed F3/F4/F5 as 1b474b4 (refactor, no behavior change). F3: tightened the image.rs Dockerfile assertion from contains() + matches('.agents').count() >= 2 to an exact matches('/home/${USERNAME}/.agents').count() == 2 with a concise failure message (no full-Dockerfile dump). F4: relocated the misplaced universal_run_args_includes_cross_harness_agents_mount test from the build_universal_data_volume_args banner to build_universal_run_args (it exercises the composed builder). F5: refreshed the two stale doc comments in run.rs (build_session_run_args now lists the .agents bind mount; run_in_container now mentions universal::prepare_host).

All 240 lib tests pass; clippy clean; no new fmt drift introduced (image.rs's only fmt flag is the pre-existing import-block drift, present on master/branch before these changes and explicitly out of scope).

Encountered a compile error mid-slice: my first F3 message string contained literal ${USERNAME} as a format-string arg to assert_eq!, and Rust 2021 implicit-format-capture read {USERNAME} as an out-of-scope variable (E0425). Fixed by escaping braces to {{USERNAME}} in the message. The .matches() argument is a plain &str (not a format string) so it correctly keeps literal ${USERNAME}. Applied via bash-level perl (temp-file exact-replace) to bypass the edit tool's format-on-write, keeping the run.rs/image.rs diffs free of the pre-existing fmt drift noise (per the b8eafcc lesson).

- **Found:** A literal ${NAME} inside a format-string argument to assert_eq! triggers Rust 2021 implicit-format-capture, yielding E0425 (cannot find value NAME). Escape as ${{NAME}}. A plain &str method argument like .matches("...") does NOT interpret braces, so it keeps literal ${NAME}.
- **Decided:** Escape the literal braces in the assert_eq! message string as {{USERNAME}} (Rust format-string escaping) rather than rewording — keeps the literal Dockerfile path visible for debugging.
- **Decided:** Use bash-level perl temp-file exact-replace for image.rs and run.rs edits to avoid the edit tool's format-on-write re-canonicalizing those files' pre-existing fmt drift (per the b8eafcc corrective-commit lesson).
- **Decided:** Bundle F3/F4/F5 into a single refactor commit: all three are no-op tightenings (assertion precision, test placement, doc-comment accuracy) and share one review rationale.

