# cast config allow/deny — Execution Todo

We are using a TDD workflow with vertical slices. For each slice, we write tests first (RED), then the minimal implementation to pass (GREEN), then commit. 

## Slice 1: Configuration Hashing
**Goal**: Compute deterministic hashes for the configuration.
- **Tests** (`src/config/approval.rs`):
  - Hash stability (same inputs → same hash)
  - Path sensitivity (different workspace paths → different hashes, use `tempfile::TempDir`)
  - Config sensitivity (different configs → different hashes)
  - HashMap ordering invariance for `extra_data_volumes`
- **Implementation**: 
  - `canonicalize_value` helper (recursive key sort for deterministic JSON)
  - `compute_config_hash(config: &Config, workspace_root: &Path) -> Result<String>`
    - Canonicalize the path via `std::fs::canonicalize` before hashing
    - Use `std::os::unix::ffi::OsStrExt::as_bytes()` for OsStr → bytes conversion
    - Null-byte separator between path bytes and config bytes
- **Validation**: `cargo test` passes.
- **Milestone**: COMMIT (GREEN)

## Slice 2: Approval Store In-Memory Logic
**Goal**: Track approved hashes in memory.
- **Tests** (`src/config/approval.rs`):
  - Approve/deny round-trip (`add_entry` → `is_approved` true; `remove_entry` → false)
- **Implementation**:
  - `ApprovalEntry` struct (Serialize, Deserialize)
  - `ApprovalStore` struct using `BTreeMap<String, ApprovalEntry>`
  - `impl ApprovalStore`: `is_approved`, `add_entry`, `remove_entry`
- **Validation**: `cargo test` passes.
- **Milestone**: COMMIT (GREEN)

## Slice 3: Approval Store Persistence
**Goal**: Save and load approvals to disk securely.
- **Tests** (`src/config/approval.rs`):
  - Persistence round-trip (`save` + `load` returns same data)
  - `load_approval_store` returns default on missing file
  - `load_approval_store` propagates error on corrupt file
- **Implementation**:
  - `approval_store_path() -> PathBuf` (via `dirs::data_dir`)
  - `load_approval_store() -> Result<ApprovalStore>`
  - `ApprovalStore::save(&self) -> Result<()>` (atomic write via `NamedTempFile` + `persist`, with `0o700` dir / `0o600` file permissions)
  - Expose module via `src/config/mod.rs` (`pub use approval::...;`)
- **Validation**: `cargo test` passes.
- **Milestone**: COMMIT (GREEN)

## Slice 4: CLI Commands (`allow` and `deny`)
**Goal**: Wire the approval logic to the user interface.
- **Implementation**:
  - Add `Allow` and `Deny` variants to `ConfigCommands` enum (with doc comments).
  - Import `get_user`, `get_workspace`, `compute_config_hash`, `load_approval_store`.
  - Implement both handlers in `src/commands/config.rs`.
- **Validation**: `cargo build` and verify manually.
- **Milestone**: COMMIT

## Slice 5: Enhance `cast config show`
**Goal**: Display current hash and status.
- **Implementation**:
  - Update `Show` handler to compute hash and check status.
  - Print the hash and approval status below the JSON output.
- **Validation**: Verify manually.
- **Milestone**: COMMIT

## Slice 6: Enforcement Gate
**Goal**: Block unapproved executions.
- **Implementation**:
  - Insert the approval check in `run_agent` (`src/dev/run.rs`) after workspace resolution, before `nix_daemon::ensure_running`.
  - Error message must include the short hash and a note about env-var overrides.
- **Validation**: Verify `cast run` fails on unapproved config, succeeds after `allow`, and fails after `deny`.
- **Milestone**: COMMIT

## Slice 7: Minimalist UI Polish (COMPLETED)
**Goal**: Simplify the UI and fix the `config show` regression.
- **Implementation**:
  - Remove hash/status display from `cast config show` (`src/commands/config.rs`).
  - Silence success output for `allow` and `deny` (`src/commands/config.rs`).
  - Remove unnecessary `.clone()` of hash (`src/commands/config.rs`).
  - Remove hash display from the enforcement gate error message (`src/dev/run.rs`).
- **Validation**:
  - Verify `cast config show` works outside a workspace.
  - Verify `cast config allow/deny` are silent on success.
  - Verify `cast run` error message is clean.
- **Milestone**: COMMIT (REFACTOR)

## Slice 8: Convert Config HashMap → BTreeMap
**Goal**: Make Config serialization natively deterministic, eliminating the canonicalization overhead.
- **Implementation**:
  - `src/config/schema.rs`: Change `agent_versions` and `extra_data_volumes` to `BTreeMap`. Update `Default` impl and `use` statement.
  - `src/config/approval.rs`: Delete `canonicalize_value`. Simplify `compute_config_hash` to use `serde_json::to_vec(config)` directly. Rename `test_hash_map_ordering_invariance` to `test_config_hash_determinism`.
  - `src/dev/volumes.rs`: Remove manual `entries.sort_by_key()` — `BTreeMap` iterates in sorted order natively.
  - `src/dev/pi/mod.rs` and `src/dev/opencode/mod.rs`: Remove unused `use std::collections::HashMap` imports.
  - `src/config/loader.rs`: No changes needed.
  - `src/dev/extra_dirs.rs`: No changes needed.
- **Note**: All existing hashes in `~/.local/share/cast/approved_configs.json` are invalidated. Accepted.
- **Validation**: `cargo test` passes. `cargo clippy` reports no warnings.
- **Milestone**: COMMIT (REFACTOR)

## Slice 9: Typestate Pattern for ApprovedConfig
**Goal**: Make the compiler enforce the approval gate — impossible to call `run_agent` or `shell` with an unapproved config.
- **Implementation**:
  - `src/config/approval.rs`:
    - Add `ApprovedConfig(Config)` newtype with private inner field.
    - Implement `Deref<Target = Config>` for transparent field access.
    - Add `into_inner(self) -> Config` method.
    - Add `#[cfg(test)] assume_approved_for_test(config: Config) -> Self`.
    - Add `ApprovalStore::verify(config: Config, workspace_root: &Path) -> Result<ApprovedConfig>`.
    - Add tests: `verify` succeeds for approved hash, fails for unapproved hash.
  - `src/config/mod.rs`: Export `ApprovedConfig`.
  - `src/commands/cli.rs`:
    - Add `verify_config(cfg: Config) -> Result<ApprovedConfig>` helper (calls `get_user`, `get_workspace`, `load_approval_store`, `store.verify`).
    - `Commands::Run`: call `verify_config(cfg)?` before `dev::run_agent`.
    - `Commands::Shell`: call `verify_config(cfg)?` before `dev::shell`.
  - `src/dev/run.rs`: Change signature to `config: &ApprovedConfig`. Remove the manual approval gate block entirely.
  - Any unit tests calling `run_agent`/`shell` directly: wrap raw `Config` with `assume_approved_for_test`.
- **Validation**: `cargo test` passes. `cargo clippy` reports no warnings. Verify `cast run` still fails on unapproved config and succeeds after `cast config allow`.
- **Milestone**: COMMIT (REFACTOR)

## Final Verification
- Run `cargo clippy` and `cargo fmt`.
- Fix any warnings or styling issues.
- **Milestone**: COMMIT (REFACTOR)
