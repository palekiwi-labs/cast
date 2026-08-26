# Project Log

## [d580f04] Spec/plan review complete; master plan saved

Reviewed the final design spec against the code (schema, build_command, run/exec/shell call paths, RunOpts presence probes, scaffold modules, flake template, volumes mount gate, stderr announcement, loader, tests, docs, changelog). All spec claims verified; no deviations found, so no decision checkpoint was raised. The slice breakdown from review was persisted as the task's master plan (no plan artifact existed before; the proposal initially lived only in chat).

- **Found:** figment Env::prefixed("CAST_") provides the four new env overrides automatically
- **Found:** announcement exists only in run_agent; exec has none
- **Found:** tests/config_test.rs:15 asserts !config.use_flake and must be updated in slice 1
- **Found:** Cargo.toml already at 0.2.0; clean-break entry targets the Unreleased changelog section
- **Decided:** 7-slice TDD sequence, one commit per slice, saved at .cue/build-repo-defined-shells/plan/index.md
- **Decided:** No master plan existed in .cue/design-repo-defined-shells either; task context is the plan home

## [3ad3310] Slice 1 complete: shell config fields added

Slice 1 committed (3ad3310). Purely additive per the compile-green sequencing decision: new config fields landed while use_flake/use_flake_path remain (temporarily serde-defaulted so the partial-JSON contract holds without naming them). Their removal moved to slice 2 together with the build_command rewrite that orphans them. TDD cycles: defaults test (RED compile -> GREEN), partial-JSON serde-defaults test (RED missing-field -> GREEN), loader merge-stack test (passed immediately; kept as end-to-end regression guard for global-vs-project precedence on the new keys). Full suite 295 tests green, clippy clean, loader.rs formatted; pre-existing fmt drift in cli.rs/build.rs/image.rs left untouched (verified pre-existing via stash).

- **Found:** cargo fmt drift exists on master in cli.rs, dev/build.rs, dev/image.rs - not caused by this task, left as-is
- **Found:** Pre-existing Config fields mostly lack serde defaults; partial-JSON fixtures must carry add_host_docker_internal and extra_data_volumes
- **Decided:** Slice 1 stays additive; legacy pair removal co-commits with slice 2 rewrite
- **Decided:** Legacy pair gains temporary #[serde(default)] so fixtures stay free of legacy keys and unchanged through slice 2
- **Decided:** Grouped all four shell fields under one 'Nix devshells' struct section; global_shell moved next to its siblings (approval hash changes once, same class as prior env_passthrough change)

## [95031e3] Slice 2 complete: verbatim ref pass-through

Slice 2 committed (95031e3). build_command rewritten to the verbatim two-layer model; agent_name param dropped and rippled through Agent trait, exec, shell; legacy use_flake/use_flake_path removed from the schema with a silently-ignored regression test (config has no deny_unknown_fields); old presence-probe tests replaced by the new behavioral spec. 296 tests green, clippy clean.

- **Found:** The edit tool's save hook reformats files with a rustfmt style that disagrees with this environment's cargo fmt (version skew: import ordering differs). Workaround: make surgical edits via perl and verify git diff is minimal before staging
- **Found:** git checkout <path> restores from the index, not HEAD; use git restore --source=HEAD --staged --worktree for full reset
- **Found:** Pre-existing cargo fmt drift on cli.rs, dev/build.rs, dev/image.rs, dev/run.rs (lines 14/612/756) confirmed present at HEAD
- **Decided:** build_command signature reduced to (config, base_command, extra_args): RunOpts became dead weight once path formatting and presence probes left
- **Decided:** shell.rs no longer calls resolve_run_opts at all (opts existed only to feed build_command)
- **Decided:** Legacy-field removal co-committed with the rewrite per the slice-1 sequencing decision
- **Open:** Slice 3 removal sweep: RunOpts.user_flake_present/project_flake_present fields, resolve_run_opts probes, scaffold call sites, announcement re-gate

## [3a07c81] 3a07c81 Slice 3 complete: implicit flake detection removed

Removed the obsolete user/project flake-presence fields and filesystem probes from RunOpts, removed automatic global flake/config scaffolding from run and exec call paths, and changed the run announcement to follow the configured global shell ref and enable switch. Added a focused config-gate regression test. Full workspace tests and clippy passed; changed Rust files are rustfmt-clean, while the repository-wide fmt check still reports only pre-existing drift in untouched files.

- **Found:** Removing the two obsolete RunOpts fields rippled through seven source/test fixture files
- **Found:** The prior HOME-mutating purity test became unnecessary once resolve_run_opts stopped probing or scaffolding flakes
- **Found:** The scaffold helper definitions remain temporarily in run.rs for slice 4, but run and exec no longer invoke them
- **Decided:** The announcement predicate is global_shell.is_some() && use_global_flake, matching command construction
- **Decided:** Kept scaffold helper definitions until slice 4 absorbs global_flake.rs and global_config.rs into cast config init
- **Open:** Slice 4: implement flagless global-only cast config init and absorb scaffold modules

## [2df53c2] 2df53c2 Initial config init path is green

Added the flagless `cast config init` command, moved the global config and flake templates into the config command, removed obsolete dev scaffold modules and run wrappers, and covered fresh initialization through the public CLI. The generated cast.json now sets the default global shell ref and retains the numtide cache trust settings. Full workspace tests and clippy passed; repository-wide fmt still reports only the known formatter-version drift, and changed files were kept free of unrelated formatting changes.

- **Found:** The branch history contains slices 1-3 behind a later merge commit, so they do not appear in the default first-parent log but are ancestors of the current branch
- **Found:** The file-edit save hook continues to apply a formatter ordering that differs from the repository environment; surgical restoration was required
- **Found:** The existing scaffold module tests were removed with their modules; public CLI coverage now verifies fresh creation
- **Decided:** Handle config init before user/workspace resolution because global bootstrap is workspace-independent
- **Decided:** Write progress notices to stderr so command output remains suitable for future stdout contracts
- **Decided:** Use ~/.config/cast/nix#default verbatim in the seeded global config
- **Open:** Add public regression coverage for never-overwrite and partial-success skip notices before declaring slice 4 complete

## [c49b63f] c49b63f Partial config init behavior covered

Added public CLI regression coverage proving `cast config init` preserves an existing global cast.json, creates the missing flake, and reports both the skip and creation on stderr. Full workspace tests, clippy, and focused rustfmt check passed.

- **Found:** The existing implementation already satisfied the partial-success behavior, so this characterization test was green immediately
- **Found:** A valid pre-existing global config is loaded before command dispatch but does not prevent init from skipping it
- **Decided:** Treat stderr notices as part of the public partial-init behavior
- **Open:** Add the symmetric existing-flake/missing-config case and verify both-existing idempotence if needed for complete never-overwrite coverage

## [e12e484] e12e484 Config init bypasses invalid config loading

Strengthened the never-overwrite contract: `cast config init` now dispatches before normal config loading, allowing it to preserve an arbitrary or malformed existing cast.json while still creating a missing flake. The public CLI test reproduces the prior failure and verifies preservation, partial creation, and notices. Full workspace tests and clippy passed; fmt output is reduced to known repository drift after formatting the newly added template constant.

- **Found:** Normal CLI startup loaded cast.json before dispatch, so a malformed user-managed file prevented partial initialization
- **Found:** The flagless init command needs no loaded Config, user, workspace, or logger state
- **Decided:** Special-case config init at the CLI boundary before load_config, preserving normal single-load behavior for all other commands
- **Decided:** An existing file is protected based on presence, regardless of whether cast can parse its contents
- **Open:** Verify the symmetric missing-config/existing-flake case before closing slice 4

## [c8b4085] c8b4085 Slice 4 behavior coverage complete

Added the symmetric partial-init regression: an existing user flake remains byte-for-byte unchanged while the missing global cast config is created, with corresponding creation and skip notices. Together with the malformed-config case, both files now have public never-overwrite and partial-success coverage. All cast crate tests, focused clippy, and focused rustfmt passed.

- **Found:** Both partial-success directions are now exercised through the real CLI with isolated HOME and data directories
- **Decided:** Slice 4's never-overwrite contract is sufficiently covered without a redundant both-files-existing test
- **Open:** Update the master plan checkbox/log state, then proceed to slice 5 template fold

## [5d09395] 5d09395 Slice 5 complete: universal folded into default

Updated the embedded global flake template so `default` includes base tooling plus opencode, pi, and claudecode. Removed the `universal` devShell while retaining each per-harness shell. Public config-init coverage verifies the generated template. Full workspace tests and clippy passed; focused rustfmt passed.

- **Found:** The template behavior can be verified through `cast config init`, keeping tests on the public bootstrap interface after removal of global_flake.rs
- **Decided:** Assert the generated shell declarations explicitly so loss of any retained per-harness shell is caught alongside the default fold
- **Open:** Slice 6: mount ~/.config/cast/nix whenever the directory exists, without requiring flake.nix

## [12d7490] 12d7490 Slice 6 complete: directory-based Nix mount

Changed the universal mount gate to bind `~/.config/cast/nix` whenever that directory exists, without requiring a flake.nix file. Added a temp-directory regression test covering an empty Nix directory. Full workspace tests and clippy passed, and the changed Rust file is rustfmt-clean.

- **Found:** The mount logic was isolated to one filter in universal volumes; no call-path changes were needed
- **Decided:** Use is_dir rather than exists so a same-named regular file cannot become an invalid bind-mount source
- **Open:** Slice 7: update the seven documentation files and Unreleased changelog clean-break entry

## [12d7490-dirty] Handoff after slices 4-6 and draft slice 7

Implementation paused at the user's request. Slices 4, 5, and 6 are committed and logged. Slice 7 documentation and changelog edits are drafted but intentionally uncommitted. The attempted sanity-reviewer handoff was cancelled, so no external review result exists. Manual stale-term scanning found one additional contradictory statement in `crates/cast/docs/commands/reference.md`, outside the seven files named by the approved plan; the user has not yet decided whether to include that eighth doc.

- **Found:** Committed slice 4 across 2df53c2, c49b63f, e12e484, and c8b4085
- **Found:** Committed slice 5 as 5d09395
- **Found:** Committed slice 6 as 12d7490
- **Found:** Draft slice 7 changes currently touch CHANGELOG.md and the seven planned documentation files
- **Found:** commands/reference.md still says cast shell uses flake detection
- **Found:** The sanity-reviewer task was cancelled and produced no findings
- **Decided:** Stop all further implementation and leave slice 7 uncommitted for the next agent
- **Decided:** Do not silently expand the seven-file documentation scope
- **Open:** Confirm whether to correct the stale commands/reference.md claim as an eighth documentation file
- **Open:** Manually review the draft documentation diff because the sanity reviewer failed
- **Open:** Run final documentation/code verification, commit slice 7, update plan/task statuses, and perform final review

## [602a93a] 602a93a Slice 7 complete: explicit shells documented

Updated the changelog and documentation for explicit global/project shell references, config init, the folded default template, and directory-based Nix mounting. With user approval, corrected the stale cast shell flake-detection claim in commands/reference.md and documented config init there. Sanity review found no issues. Workspace tests and clippy passed; cargo fmt --check continues to report only pre-existing formatter-version drift in untouched Rust files.

- **Found:** The eighth documentation file, commands/reference.md, contained a contradictory automatic flake-detection claim
- **Found:** Sanity review found no documentation or changelog issues
- **Found:** Workspace tests passed: 344 total across unit, integration, and client suites
- **Found:** Clippy passed with warnings denied
- **Found:** Repository-wide rustfmt check still fails on pre-existing drift in untouched cast and cast-mcp-client Rust files
- **Decided:** Include commands/reference.md in slice 7 with explicit user approval
- **Decided:** Document config init in the command reference while correcting its stale shell behavior
- **Decided:** Treat git diff --check plus no changed Rust files as the formatting gate for this docs-only slice
- **Open:** Update master plan and task statuses, then perform final branch review

## [602a93a] Review: build-repo-defined-shells branch diff

Reviewed full branch diff against master. All 344 tests pass. Key findings recorded below.

- **Found:** Tilde in DEFAULT_CAST_JSON global_shell is passed verbatim to nix develop inside the container — relies on nix develop performing shell expansion. The docs acknowledge this but there is no test or runtime guard for when HOME is unset inside the container.
- **Found:** build_command calls global_layer(config) and project_layer(config) twice each — once for capacity estimation and once for the actual value. This is correct and cheap for String clones but slightly redundant.
- **Found:** Missing integration-level test for CAST_GLOBAL_SHELL / CAST_PROJECT_SHELL / CAST_USE_GLOBAL_FLAKE / CAST_USE_PROJECT_FLAKE env var overrides (documented in CHANGELOG and env-overrides.md but no subprocess test exercises them).
- **Found:** cast config init idempotency (both files already present) not covered by an integration test — the two partial-idempotency tests cover one-file-present cases but not both-present.
- **Found:** unreachable! arm at config.rs:105 is dead code by design (Init is caught twice before reaching the match). Acceptable as a safety net but adds noise.
- **Found:** project_shell announce path: the global devshell announcement fires in run_agent but there is no symmetric announcement for project_shell. This is intentional (shellHook is relied upon), but not documented in the user-facing docs.
- **Found:** Removal of scaffold_global_flake / scaffold_global_cast_json from run_agent and exec is a deliberate breaking change, documented in CHANGELOG. First-run experience now requires explicit cast config init.
- **Decided:** No blocking correctness issues found. All test suites pass.
- **Open:** Whether nix develop respects ~/… tilde expansion inside the container when HOME is set to the agent user home

## [b91d63e] b91d63e Final review stale guidance fixed

Addressed final review findings in the root quick-start and embedded flake comments. The quick-start now initializes global config explicitly and describes the configured default shell; template comments no longer claim first-run seeding or agent-name shell selection. Focused config-init tests passed.

- **Found:** Final review found stale setup guidance outside the original documentation list
- **Found:** The embedded template comments still described removed first-run and agent-name selection behavior
- **Decided:** Fix incorrect docs and template comments under the user's explicit instruction
- **Decided:** Do not add redundant both-existing init coverage; both partial directions already prove preservation and the plan records this decision
- **Decided:** Keep tilde refs verbatim as required by the approved specification and existing behavioral test; expansion would violate the explicit interface
- **Open:** Re-run final review/status and close plan/task artifacts

## [dfa03d5] dfa03d5 Shell environment overrides covered

Extended the public config-show integration test to exercise all four new shell environment overrides. The focused test and rustfmt check passed.

- **Found:** Figment correctly maps the documented CAST_GLOBAL_SHELL, CAST_PROJECT_SHELL, CAST_USE_GLOBAL_FLAKE, and CAST_USE_PROJECT_FLAKE variables
- **Decided:** Address the final review's env-override coverage gap with the existing public subprocess test
- **Decided:** Retain the prior decision that symmetric partial-init tests sufficiently establish never-overwrite behavior without a redundant both-existing test
- **Open:** Close plan and task after final clean-state check

## [dfa03d5] Repo-defined shells task complete

All seven planned slices are implemented, committed, documented, and reviewed. Final review findings were resolved or explicitly dispositioned. The working tree is clean, the task and master plan are marked complete, workspace tests and clippy pass, and changed Rust files pass focused rustfmt checks.

- **Found:** Final clean-state check reports no uncommitted repository changes
- **Found:** No unresolved high-severity correctness findings remain
- **Found:** The only repository-wide formatting failure is known pre-existing rustfmt-version drift in untouched files
- **Decided:** Close the build task and master plan
- **Decided:** Leave fully-idempotent config-init coverage unadded because both partial-success directions already prove preservation and creation semantics

## [dfa03d5] Agent shell naming reconsidered for service pivot

Analyzed the reopened naming question against the current explicit-shell implementation and the herdr service pivot. `global_shell` is misleading because project config can override it and the layer is not intrinsically global. `agent_shell` describes the current harness-provisioning role better, but would become too narrow under the pivot because the same outer environment must also provide service infrastructure such as the PID-1 multiplexer. The durable concept is the sandbox/service baseline environment, with `project_shell` layered inside it.

- **Found:** The current default shell already contains all harnesses rather than defining one agent
- **Found:** The service pivot composes the outer shell around the multiplexer server command, so the layer will provide infrastructure as well as agents
- **Found:** Configuration merge semantics allow a repository to override the current global_shell, weakening the meaning of global
- **Found:** The pivot explicitly aims to dissolve hardcoded agent taxonomy, making agent_shell strategically inconsistent
- **Decided:** Recommend against renaming global_shell to agent_shell
- **Decided:** Prefer a role-based name such as sandbox_shell for the outer baseline environment; base_shell is a shorter alternative
- **Decided:** Reserve service_shell until the service model is approved, because current 0.2 sessions are not yet services
- **Open:** Choose whether to rename now to sandbox_shell/use_sandbox_flake, retain global_shell through 0.2, or adopt another role-based term
- **Open:** Decide whether the boolean should remain flake-oriented or mirror the shell concept more directly

## [dfa03d5] Flash recommends sandbox shell naming

Consulted Gemini Flash on the outer shell naming decision. It independently rejected agent_shell as coupling the schema to the retiring single-agent model and rejected global_shell as conflating config scope with execution role. It recommends sandbox_shell for the outer runtime environment and project_shell for repository tooling.

- **Found:** sandbox_shell remains semantically accurate for both current sandbox sessions and the planned long-lived service model
- **Found:** A project-level override of sandbox_shell reads coherently, unlike global_shell
- **Found:** base_shell risks confusion with the container base image
- **Found:** agent_shell would likely require another rename during the 0.3 pivot
- **Decided:** Consultant recommendation: rename global_shell to sandbox_shell before the 0.2 API freeze
- **Decided:** Consultant also recommends use_sandbox_shell and use_project_shell so toggle nouns match the fields they control
- **Open:** Operator decision on adopting both the shell field rename and boolean rename

## [dfa03d5] Decision: rename outer layer to sandbox shell

The operator approved the role-based naming before the 0.2 API freeze. Rename global_shell to sandbox_shell, use_global_flake to use_sandbox_shell, and use_project_flake to use_project_shell across schema, environment overrides, command construction, bootstrap output, tests, documentation, and changelog.

- **Decided:** Adopt sandbox_shell as the outer sandbox/runtime environment ref
- **Decided:** Rename toggles to use_sandbox_shell and use_project_shell so they match the controlled fields
- **Decided:** Treat the rename as a clean break before the 0.2 configuration API freezes

## [95a0556] 95a0556 Sandbox shell rename complete

Implemented the approved clean-break naming across schema, command construction, config init, announcements, tests, docs, template comments, and changelog. The outer layer is now sandbox_shell with use_sandbox_shell; the project toggle is use_project_shell. Environment overrides follow the same names. Global terminology remains only where it accurately denotes the global configuration-file scope or historical changelog entries.

- **Found:** The edit save hook again introduced formatter-version import drift; surgical restoration kept the commit limited to semantic rename changes
- **Found:** Sanity review initially conflated global config-file scope with sandbox execution-layer scope; the final terminology preserves that distinction
- **Found:** No stale old field or environment variable references remain in tracked source and current documentation
- **Decided:** Rename the user-visible loading notice to 'Loading sandbox nix devshell...'
- **Decided:** Keep init_global_config and 'global cast config' because they describe configuration location, not execution-layer role
- **Decided:** Do not rewrite historical 0.2.0 changelog entries that accurately record the old API at that release point
- **Open:** Mark slice 8 complete after final clean-state verification

