---
status: complete
refs:
- /home/pl/code/palekiwi-labs/cast/.cue/master/task/cleanup-post-nix-native-harnesses.md
- /home/pl/code/palekiwi-labs/cast/.cue/cleanup-post-nix-native-harnesses/trace/1784916739-c6794d5/dead-code-scan.md
---
# Executive Plan: Cleanup Post Nix-Native Harnesses

## Foreword

The `nix-native-harnesses` pivot (merged, v0.2.0) already deleted the
harness auto-update/version-check machinery (the entire `dev/version/` module,
`version_cache_ttl_hours`, and per-agent image build logic) in commits
`18c54e2` and `8cce186`. This cleanup task removes the remaining orphans the
pivot left behind, identified in the dead-code scan trace.

Branch: `chore/cleanup-post-nix-native-harnesses` (off `master`).

Scope is two phases: non-breaking hygiene first, then a user-facing CLI
breaking change (collapsing `cast build <agent>` into a single `cast build`).

## Phase 1 - Non-breaking hygiene

- [x] Remove orphaned `ureq` dependency from `crates/cast/Cargo.toml:24`
      (was used only by the deleted version fetcher; confirmed zero usages).
- [x] Fix stale doc comment in `src/dev/run.rs:61-62` referencing the
      non-existent `Agent::extra_run_args` (now `config_mount_args` /
      `env_passthrough_args`).
- [x] Fix stale comment in `src/nix_daemon/daemon.rs:23` ("dynamic image tag
      based on assets hash" - tag now derives from `CAST_VERSION`).
- [x] Simplify the `Agent` trait doc note in `src/dev/agent.rs:16-18` to
      describe current state (drop the "no longer" transition framing).
- [x] Verify `cargo build` + `cargo test -p cast` green; commit.

## Phase 2 - Collapse per-agent `cast build` (BREAKING CLI change)

Rationale: there is one shared harness-free dev image. `cast build opencode`,
`cast build pi`, `cast build claudecode` all dispatch to identical logic. The
agent subcommand is meaningless for build.

- [x] RED: update `tests/cli_test.rs` build tests to the new surface -
      `cast build --help` shows flags (`--base`/`--force`/`--no-cache`) and no
      agent subcommand; remove `test_cast_build_opencode_help`.
- [x] GREEN: replace `Commands::Build { agent: BuildAgent }` with
      `Commands::Build` carrying `--base`/`--force`/`--no-cache` flags directly;
      delete the `BuildAgent` enum; collapse the three dispatch arms in
      `cli.rs` into one.
- [x] Drop the unused `_agent: &dyn Agent` parameter from
      `dev::build_agent` (`src/dev/build.rs:15`) and update the single caller.
- [x] **Bonus:** Rename `--base` -> `--nix-daemon` on `cast build`.
- [x] Verify `cargo build` + `cargo test -p cast` green; commit.

## Phase 3 - Docs & changelog

- [x] Add a CHANGELOG entry noting `cast build <agent>` collapsed to
      `cast build`, flag renamed, and `ureq` removed.
- [x] Update `reference.md` command docs to reflect single `cast build` and
      `--nix-daemon` flag.
- [x] Fill task acceptance-criteria evidence; request human QA attestation.

## Out of scope (keep)

- Backward-compat serde tests in `config/schema.rs:289-314` (they guard old
  `agent_versions`/`universal_container` config keys - intentional).
