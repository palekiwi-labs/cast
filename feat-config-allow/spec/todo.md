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

## Final Verification
- Run `cargo clippy` and `cargo fmt`.
- Fix any warnings or styling issues.
- **Milestone**: COMMIT (REFACTOR)
