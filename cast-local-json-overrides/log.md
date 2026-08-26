# Project Log

## [3a07c81] Research complete, worktree ready, master plan written

Prepared the cast.local-json-overrides task: created worktree, verified the environment, and wrote the master plan before implementation.

Key research results driving the plan:
- The only code that reads the project cast.json is load_config_with_global in crates/cast/src/config/loader.rs; everything else referencing cast.json is comments or global config scaffolding.
- figment 0.10.19 registry source confirms Json::file with an absolute path silently yields an empty provider when the file is missing, and dict merges are deep per key while arrays replace wholesale.
- The config approval system hashes the serialized effective Config, so local overrides will intentionally invalidate prior approvals and force re-approval via `cast config allow`.
- Master and build-repo-defined-shells have identical loader merge chains, so the new branch build-cast-local-json was based off master to keep the change independent.

Worktree worktrees/build-cast-local-json created; baseline `cargo test -p cast --lib config::loader` is green (13 passed).

- **Found:** figment 0.10.19 Json::file: absolute path + missing file = empty provider (no-op)
- **Found:** figment merge semantics: dicts deep-merge per key, arrays replace wholesale (matches existing env_passthrough tests)
- **Found:** Approval system (config/approval.rs) hashes the full effective Config, so local overrides flip approval status to Changed
- **Found:** crates/cast/docs/config/overview.md holds the precedence lists that need updating
- **Decided:** Base branch off master (d580f04) since the loader chain is identical and the feature is independent of build-repo-defined-shells
- **Decided:** Implement as a single figment merge layer between cast.json and the cast-mcp.json mcp merge
- **Decided:** Treat missing cast.local.json as a no-op and malformed as an error, matching existing file layers
- **Decided:** Approval hash must change when local overrides apply (re-approval required) — kept as a security property, documented in docs
- **Open:** Whether the `Changed` approval error message should also mention cast.local.json (planned as optional slice 8)

## [3a07c81] Committed local config merge tracer

Completed the tracer-bullet TDD cycle. The new unit test first failed with project memory 2048m instead of local memory 4096m. Added the minimal figment merge layer for cast.local.json immediately after cast.json; the targeted test and all 14 loader tests pass. Committed as 28f5a1a (`feat: add local config override`).

Pre-commit validation passed for the changed file: targeted rustfmt, clippy with warnings denied, cargo check, and loader tests. Repository-wide cargo fmt --check is baseline-red under rustfmt 1.94 because it wants unrelated import/line wrapping changes across both crates; no unrelated formatting was applied.

- **Found:** Tracer test failed before implementation and passed after the single figment merge line
- **Found:** All 14 config loader unit tests remain green
- **Found:** Repository-wide cargo fmt --check has unrelated baseline diffs under rustfmt 1.94
- **Decided:** Place cast.local.json after cast.json and before cast-mcp.json, preserving the planned precedence chain
- **Decided:** Use targeted rustfmt validation because repo-wide formatting is incompatible with the installed newer rustfmt and would rewrite unrelated files

## [3a07c81] Committed local precedence coverage

Added and ran confirmation tests one behavior at a time after the tracer implementation. Verified project-key fallthrough, missing-file no-op, cast-mcp.json precedence over local MCP settings, and array replacement semantics. All tests passed immediately as expected from the figment layer. Committed as db0dbb9 (`test: cover local config precedence`).

- **Found:** cast.local.json preserves project values for keys it does not specify
- **Found:** Missing cast.local.json leaves project behavior unchanged
- **Found:** cast-mcp.json remains higher precedence for MCP keys
- **Found:** Arrays in cast.local.json replace project arrays rather than concatenate
- **Decided:** Keep four explicit behavior tests rather than relying on figment implementation assumptions
- **Decided:** Batch confirmation-only tests into one atomic test commit, per the approved plan

## [3a07c81] Committed CLI local config coverage

Added CLI integration coverage using isolated workspace and data tempdirs. `cast config show` now verifies cast.local.json overrides project cpus while CAST_MEMORY still overrides the local memory value. All 10 config integration tests pass. Committed as 94e3ef1 (`test: verify local config in CLI`).

- **Found:** The effective CLI config reports local cpus 2.5 over project 1.5 and env memory 8g over local 4g
- **Decided:** Use one subprocess test to verify both local-over-project and env-over-local precedence through the public CLI
- **Decided:** Route approval/log side effects to temp directories for nix sandbox compatibility

## [3a07c81] Committed local config approval coverage

Added an end-to-end approval test: approve a workspace with cast.json, add cast.local.json with a changed memory value, then verify `cast config show` reports the effective local value and emits the `cast config diff` changed-config hint. All 11 config integration tests pass. Committed as 7f9acb1 (`test: require local config approval`).

- **Found:** Adding cast.local.json after approval produces the existing Changed approval status and diff hint automatically
- **Decided:** Treat cast.local.json changes as effective-config changes requiring explicit reapproval
- **Decided:** Test through the public CLI and isolated approval store rather than internal hash implementation

## [3a07c81] Committed local approval guidance

Completed a red-green cycle for approval UX. Added an assertion that changed-config errors identify cast.local.json; it failed against the env-vars-only note. Updated the note to mention both cast.local.json and CAST_* overrides. All 30 approval tests pass. Committed as b8df350 (`fix: mention local config approval`).

- **Found:** Existing changed-config guidance mentioned only env vars despite local files now also invalidating approval
- **Decided:** Explicitly identify cast.local.json in the approval error because local overrides affect the same effective-config hash as environment overrides

## [3a07c81] Committed local config documentation

Documented cast.local.json across the configuration overview, field reference, env-list replacement guidance, loader precedence comment, and changelog. Added cast.local.json to the repository .gitignore and told downstream projects to do the same before creating personal overrides. Documented the reapproval requirement. Committed as ddfff2c (`docs: document local config overrides`).

- **Found:** The env override guide had wording tied to two config files; generalized it to all config files and added the local replacement behavior
- **Decided:** Recommend that projects explicitly gitignore cast.local.json; also ignore it in this repository
- **Decided:** Document array replacement and approval semantics next to the new precedence layer
- **Decided:** Add the feature to the Unreleased changelog

## [3a07c81] Committed malformed local config coverage

Added explicit coverage that a present but malformed cast.local.json fails configuration loading with the existing `Failed to load configuration` context. This complements the missing-file no-op test and matches cast.json behavior. All 19 loader tests pass. Committed as 12a86a5 (`test: reject malformed local config`).

- **Found:** The direct figment Json merge surfaces malformed local JSON through the existing extraction context as intended
- **Decided:** Malformed cast.local.json is a hard configuration error; only absence is silently ignored

## [3a07c81] Addressed sanity review test isolation

Addressed both minor sanity-review recommendations. Refactored local loader fixtures through `load_with_project_files`, which removes duplication and calls `load_config_with_global(..., None)` so tests cannot read user/global configuration. Routed the new CLI test's log directory under its test-owned TempDir. Clippy all targets and loader/config integration tests remain green. Committed as 6c5e70d (`test: isolate local config fixtures`).

- **Found:** The first test version used load_config_from, which could consult the user's global config even though project fixtures were temporary
- **Found:** Sanity review found no functional or documentation defects
- **Decided:** Isolate local-config unit tests from real global config by passing global_path None
- **Decided:** Keep all new filesystem/log side effects under test-owned temporary directories

## [3a07c81] Completed cast.local.json overrides

Implementation complete on branch build-cast-local-json in worktrees/build-cast-local-json. Final post-review gate passed with a clean worktree: changed-file rustfmt check, `cargo clippy -p cast --all-targets -- -D warnings`, `cargo build -p cast`, `cargo test -p cast`, and `git diff --check`.

Test totals: 270 unit tests, 18 CLI integration tests, and 11 config integration tests passed (299 total; doc tests also passed). Sanity review found no functional issues; both minor test-isolation recommendations were addressed in commit 6c5e70d.

- **Found:** Final branch contains 8 atomic commits and has no uncommitted changes
- **Found:** All 299 cast tests pass after review-driven test isolation refactor
- **Decided:** Mark task and plan complete after all acceptance behavior, documentation, review feedback, and final verification passed

## [3a07c81] Completed independent branch diff reviews

Saved the complete `master...build-cast-local-json` diff at head `6c5e70d` and requested independent reviews from Gemini Flash and Opus. Flash found no actionable issues. Opus found no correctness defect in the implementation but identified documentation and test-strength improvements. Review outputs are saved beside the branch diff in the task trace.

- **Found:** Core merge order correctly implements defaults < global < project < local < MCP < environment precedence.
- **Found:** Gemini Flash reported no actionable findings.
- **Found:** Opus reported two medium documentation gaps, one medium malformed-error assertion weakness, one medium-low map-merge coverage gap, and five low-priority maintainability/documentation concerns.
- **Found:** Opus reported relevant tests and clippy passing; repository-wide formatting failures were pre-existing on master.
- **Open:** Decide whether MCP and approval documentation gaps should block completion.
- **Open:** Strengthen malformed local JSON error assertion and add extra_data_volumes map-merge coverage.
- **Open:** Triage low-priority fixture and documentation refinements.

