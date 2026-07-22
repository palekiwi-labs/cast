# Project Log

## [e87338d] Research complete: Config diff support for approvals

I researched the current implementation of the config approval system in `cast` to evaluate the feasibility of adding diff support. 

Key discoveries:
- Configs are currently tracked only by their SHA256 hashes.
- `ApprovalEntry` does not store the configuration content, making diffing impossible with the current schema.
- `cast config show` only outputs the current JSON config.

I have documented the required schema and logic changes in `.mem/feat-config-diff/doc/config-diff-research.md`.

- **Found:** Approvals are stored in ~/.local/share/cast/approved_configs.json.
- **Found:** The system uses a BTreeMap indexed by hash, which complicates workspace-based lookup of the "last" approved config.
- **Decided:** The ApprovalEntry schema needs to be updated to include the serialized configuration content.
- **Decided:** cast config show should be enhanced to perform workspace-based lookup of previous approvals.

## [e023b62-dirty] Commit 1: add similar and owo-colors deps

- **Found:** similar 2.7.0 and owo-colors 4.3.0 added to crates/cast/Cargo.toml
- **Decided:** CAST_DATA_DIR will be committed separately with the approval.rs schema changes it enables

## [fcb5248-dirty] Commit 2: ApprovalEntry schema + snapshot + workspace lookup

- **Found:** All 171 unit tests pass after schema change
- **Found:** Existing test call sites updated to pass serde_json::Value::Null as placeholder snapshot
- **Decided:** CAST_DATA_DIR change included here as it is part of the approval store infrastructure

## [134829d-dirty] Commit 3: format_config_diff module

- **Found:** format_config_diff returns plain text unified diff (no ANSI codes) using similar::TextDiff
- **Found:** 3 unit tests pass: changed value, identical values, added field

## [ed8f6d9] Commit 4: cast config diff subcommand — feature complete

- **Found:** All 9 config integration tests pass (5 new)
- **Found:** cast config show emits a stderr hint when unapproved (stdout JSON contract preserved)
- **Found:** cast config diff handles all 4 cases: no entry, legacy entry (no snapshot), no changes, changed config with colored diff output
- **Decided:** Show hint and Diff subcommand committed together as one logical unit — both are CLI layer changes in commands/config.rs

## [2ee0c25] Remove legacy config approval support

Removed compatibility for legacy ApprovalEntry objects that lacked the approved_config snapshot. Since the project is in active development and this feature is new, we chose to simplify the schema and logic by making the snapshot mandatory.

- Updated ApprovalEntry to have a mandatory approved_config field.
- Simplified Diff command handler to remove legacy entry handling.
- Removed legacy tests and updated existing tests to use a fresh CAST_DATA_DIR to ensure isolation.
- Verified all 199 tests pass.

- **Found:** Existing tests in cli_test.rs were vulnerable to existing data in the environment, fixed with CAST_DATA_DIR isolation.
- **Decided:** Eliminate legacy config approval support to simplify codebase.
- **Decided:** Make approved_config a mandatory field in ApprovalEntry.

## [070d3f8] Use compact JSON for approval store

Switched the serialization of the ApprovalStore from pretty-printed to compact JSON. Since the config diff logic already handles pretty-printing in memory for the comparison, storing the snapshots in a compact format saves disk space without affecting the quality of the user-visible diff.

- **Found:** Config diff output is unaffected by the storage format as it re-formats to pretty-printed JSON before diffing.
- **Decided:** Store approval snapshots in compact JSON format to improve storage efficiency.

