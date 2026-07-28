# Project Log

## [c6794d5] Research complete: dead code scan post-nix-native-harnesses

Conducted a thorough scan of the codebase for dead code post-nix-native-harnesses pivot.

Key findings:
- `ureq` dependency in `crates/cast/Cargo.toml` is completely unused.
- `Agent::extra_run_args` references in `run.rs` documentation are outdated.
- `cast build <agent>` CLI surface is redundant as all agents share the same image and build logic.
- Outdated comment in `nix_daemon/daemon.rs` regarding image tag being based on assets hash.
- Legacy fields `agent_versions` and `universal_container` are explicitly ignored in tests, which is fine but worth noting.
- Doc comment in `Agent` trait mentions old behavior.

Full list of findings prepared for the report.

- **Found:** ureq dependency is dead in crates/cast/Cargo.toml
- **Found:** Agent::extra_run_args documentation reference is dead in crates/cast/src/dev/run.rs
- **Found:** build_agent takes unused _agent parameter in crates/cast/src/dev/build.rs
- **Found:** Redundant per-agent build CLI surface in crates/cast/src/commands/cli.rs
- **Found:** Outdated comment in crates/cast/src/nix_daemon/daemon.rs regarding assets hash tagging

## [ab1d38e] Phase 1: drop dead ureq dep + stale comments

Committed ab1d38e on branch chore/cleanup-post-nix-native-harnesses. Non-breaking hygiene from the dead-code scan.

- **Found:** ureq compiled away cleanly - confirms it was truly orphaned (only used by the deleted dev/version fetcher)
- **Found:** Auto-update/version-check machinery was already fully removed in the original story (commits 18c54e2, 8cce186); no logic remained, only the ureq dep and stale comments
- **Decided:** Keep backward-compat serde tests in config/schema.rs:289-314 (intentional guards for old config keys)

## [4636b77] Phase 2: collapse per-agent cast build (breaking)

Committed 4636b77 on branch chore/cleanup-post-nix-native-harnesses. TDD red-green: updated cli_test build tests first, watched them fail, then implemented.

- **Found:** Two CLI tests referenced the per-agent build surface: test_cast_build_opencode_help and test_cast_build_claudecode_help - both removed
- **Decided:** cast build <agent> collapses to a single cast build with --base/--force/--no-cache flags; BuildAgent enum deleted; dev::build_agent loses its unused agent param
- **Decided:** Marked as BREAKING CHANGE (refactor!) since it is a user-facing CLI change

## [67d208c] Rename cast build --base to --nix-daemon + fix stale docs

Committed 67d208c. TDD red-green on cli_test. User-approved flag rename.

- **Found:** Dev image is NOT built FROM the nix-daemon image (image.rs uses standalone Dockerfile.dev) - the two are independent images, so --base was a genuine misnomer
- **Found:** reference.md docs still described 'build <agent>' with a CLI-symmetry note - stale from the Phase 2 collapse; fixed in same commit
- **Found:** param name nix_daemon does not collide with the nix_daemon module import (Rust value vs module namespaces are separate)
- **Decided:** Renamed --base -> --nix-daemon to align with the cast nix-daemon subcommand; left --force/--no-cache unchanged

## [eb15a8e] Rename build_agent to build_dev_image (review feedback)

- **Found:** Both diff reviewers independently flagged build_agent as carrying retired vocabulary; one call site, low-cost rename
- **Decided:** Renamed to build_dev_image to match the doc comment and the post-pivot mental model; item 2 (negative CLI tests) deferred per user direction

