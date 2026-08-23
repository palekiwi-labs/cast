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

