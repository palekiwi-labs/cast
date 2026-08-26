# Project Log

## [d86c37f] Release 0.2.0 coordination task created

Created the coordination task card .cue/master/task/release-0.2.0.md and its master plan .cue/release-0.2.0/plan/index.md, both linking the two tagged stories via refs and a story tracker.

- **Found:** CHANGELOG.md exists (Keep a Changelog format) and already contains a [0.2.0] - 2026-07-24 section written at the universal-container merge, with an [Unreleased] section accumulating above it. The 0.2.0 entry was never tagged, so at release time Unreleased folds into the 0.2.0 section rather than starting a new version.
- **Found:** Scope reality: tagging 0.2.0 is converge-and-cut, not bump-and-cut; the Cargo.toml bump already happened.
- **Decided:** Task kind coord, priority high; owns scope convergence, story tracking, and the release gate (changelog, version audit, tag, push). Implementation stays out of scope.
- **Decided:** Story tracker treats env-var-passthrough as immediately actionable and design-repo-defined-shells as gating its own spawned build tasks; the two are sequenced independently.
- **Open:** Whether cast-mcp-client rides at 0.1.0 or bumps with the release.
- **Open:** Deferred repo-wide formatting commit (pre-existing rustfmt mismatches) must be decided before the tag.
- **Open:** Any further stories pulled into 0.2.0 are a user decision recorded in this task.

## [95a0556] Simplified release task and plan

Removed the deferred herdr 0.3.0 work from the release task references, updated cast.local.json overrides as merged in PR #65, and reduced the plan to the active scope and release gate.

- **Found:** cast-local-json-overrides is already marked complete and merged to master in PR #65
- **Found:** build-repo-defined-shells is the only unfinished 0.2.0 story currently listed
- **Decided:** Keep historical implementation detail out of the live release plan
- **Decided:** Track completed stories as concise release-scope checkpoints
- **Decided:** Keep the plan focused on remaining merge and release mechanics

## [71a6343] Added Renovate setup and dependency update tasks to 0.2.0 release scope

Updated the release master plan and task card to reflect build-repo-defined-shells completion and incorporated setup-renovate and update-dependencies-0.2.0 into the active release scope.

- **Found:** build-repo-defined-shells has merged to master
- **Found:** setup-renovate and update-dependencies-0.2.0 were added as active 0.2.0 child tasks
- **Decided:** Track setup-renovate and update-dependencies-0.2.0 as active scope items for 0.2.0
- **Decided:** Mark build-repo-defined-shells as completed and merged to master in release plan and task card
- **Open:** Execution and merge of setup-renovate and update-dependencies-0.2.0
- **Open:** cast-mcp-client versioning decision
- **Open:** Release gate execution (changelog, formatting/clippy/tests, tagging)

