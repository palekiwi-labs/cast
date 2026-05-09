# Project Log

## [8e4aef7] Research complete: cast config allow/deny

- **Found:** Config fields in src/config/schema.rs identified for hashing
- **Found:** Interception point for approval check found in src/dev/run.rs#run_agent
- **Found:** Proposed persistence in ~/.local/share/cast/approved_configs.json

## [58e35e1] Implementation plan complete: cast config allow/deny

- **Decided:** Hash includes workspace root path (per-project approval)
- **Decided:** All Config fields are hashed (full snapshot)
- **Decided:** Approval store at ~/.local/share/cast/approved_configs.json
- **Decided:** Canonicalize HashMap serialization by sorting JSON object keys recursively
- **Decided:** Interception point in run_agent before nix_daemon::ensure_running
- **Decided:** New module: src/config/approval.rs

## [58e35e1] Plan updated with Gemini review findings

- **Found:** as_encoded_bytes() does not exist — must use OsStrExt::as_bytes() on Unix
- **Found:** workspace path must be canonicalized with std::fs::canonicalize before hashing
- **Found:** ApprovalStore.entries must use BTreeMap not HashMap for deterministic JSON
- **Found:** load_approval_store must distinguish NotFound from other IO errors
- **Decided:** File permissions: parent dir 0o700, store file 0o600
- **Decided:** cast config show enhanced to display hash and approval status
- **Decided:** Error message in run_agent includes short hash and env-var override note
- **Decided:** cast config deny removes only the current hash (not all workspace approvals)

## [486f1d4] Slice 1: Configuration Hashing complete

- **Found:** Canonicalize config by sorting JSON keys
- **Found:** Hash includes canonicalized workspace path
- **Decided:** Use sha2 and OsStrExt for hashing

## [1976974] Slice 2: Approval Store In-Memory Logic complete

- **Found:** is_approved, add_entry, remove_entry implemented
- **Found:** Time-based approval tracking

## [ec9ac88] Slice 3: Approval Store Persistence complete

- **Found:** Atomic JSON persistence with 0o600 permissions
- **Found:** Parent directory created with 0o700
- **Found:** Handled missing and corrupted store files

## [03dd0be] Slice 4: CLI Commands complete

- **Found:** cast config allow and deny subcommands implemented
- **Found:** Handlers use shared hashing and storage logic

## [6660d28] Slice 5: Enhance cast config show complete

- **Found:** Hash and Status displayed below JSON output
- **Found:** Status check uses the shared approval store

## [204d6ed] Slice 6: Enforcement Gate complete

- **Found:** Blocked unapproved executions in run_agent
- **Found:** Enhanced rejection message with hash and env-var note
- **Found:** Updated config show to print hash/status to stderr for JSON compatibility

## [0c3b20d] Slice 7: Minimalist UI Polish complete

- **Found:** Removed all user-facing hash displays from CLI output to keep it internal.
- **Found:** Silenced `cast config allow/deny` on success (no output).
- **Found:** Simplified `cast config show` to only display the configuration, fixing the workspace regression.
- **Found:** Removed hash from the enforcement error message in `run_agent`.
- **Found:** Removed unnecessary `.clone()` in `src/commands/config.rs`.

## [PLANNING] Architectural refinements: Typestate + BTreeMap

- **Decided:** Rejected moving approval logic to `impl Config` — Config must remain a pure data struct (DTO).
- **Decided:** Adopt the Typestate Pattern: introduce `ApprovedConfig(Config)` newtype; the only constructor is `ApprovalStore::verify()`.
- **Decided:** `dev::shell` must also require `ApprovedConfig` — same security boundary as `run_agent`.
- **Decided:** Approval check lifted out of `run_agent` into a `verify_config()` helper in `src/commands/cli.rs`.
- **Decided:** `Deref<Target = Config>` on `ApprovedConfig` so all internal field accesses remain unchanged.
- **Decided:** Add `#[cfg(test)] ApprovedConfig::assume_approved_for_test()` escape hatch for unit tests.
- **Decided:** Convert `Config` HashMap fields (`agent_versions`, `extra_data_volumes`) to `BTreeMap` for native deterministic serialization.
- **Decided:** Delete `canonicalize_value` and the intermediate JSON DOM step once BTreeMap is in place.
- **Decided:** Hash invalidation from the BTreeMap migration is acceptable — users run `cast config allow` once more.
- **Decided:** Figment deserialization is unaffected by HashMap → BTreeMap (operates on its own internal Dict, confirmed).
- **Decided:** Do not pre-compute the hash at load time — lazy computation is correct; pre-computing wastes effort on commands that don't need approvals.

## [b7562fa] Slice 8: Convert Config HashMap → BTreeMap

- **Found:** Native determinism with BTreeMap simplifies hashing logic
- **Decided:** Removed canonicalize_value as Config is now natively deterministic

## [c5c1cb2] Slice 9: Typestate Pattern for ApprovedConfig

- **Found:** Typestate pattern ensures compile-time security gate
- **Decided:** Moved approval logic from execution core to CLI dispatcher

## [81e1274] Final Polish and Audit

- **Found:** No issues found during audit; one minor clippy warning fixed
- **Decided:** Finalizing implementation of config approval gate

## [70a3d9e] Extend security gate and refine revocation logic

Completed a security audit of the config approval system. Extended the ApprovedConfig typestate to build and nix-daemon commands to prevent unapproved Nix/container operations. Updated the deny command to revoke all workspace-associated approvals for better security. Enabled pretty-printing for the approval store JSON.

- **Found:** nix-daemon and build commands were bypassable with raw Config
- **Decided:** Moved all container-initiating commands to require ApprovedConfig

