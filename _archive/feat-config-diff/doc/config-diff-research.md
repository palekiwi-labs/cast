# Research Report: Config Diff Support for Approvals

This report documents the current implementation of the configuration approval system in `cast` and outlines the changes required to support diffing the last approved configuration against the current state.

## Research Questions Answered
1. How does the current config approval system track and verify configurations?
2. Where are approvals stored and what information do they contain?
3. What is needed to enable diffing between the last approved and current configurations?

## 1. Current Approval System

The approval system is implemented in `crates/cast/src/config/approval.rs`. It uses a "gatekeeper" pattern where sensitive operations require an `ApprovedConfig` type, which can only be obtained by verifying a `Config` against the `ApprovalStore`.

### Hashing Mechanism
A SHA256 hash is computed from the canonicalized workspace root and the serialized JSON representation of the `Config`.

**Source:** `crates/cast/src/config/approval.rs`
```rust
pub fn compute_config_hash(config: &Config, workspace_root: &Path) -> Result<String> {
    // ...
    let config_bytes = serde_json::to_vec(config)?;

    let mut hasher = Sha256::new();
    hasher.update(canonical_root.as_os_str().as_bytes());
    hasher.update(b"\0");
    hasher.update(&config_bytes);

    Ok(hex::encode(hasher.finalize()))
}
```

### Persistence
Approvals are stored in `~/.local/share/cast/approved_configs.json`. The `ApprovalStore` maintains a map of these hashes.

**Source:** `crates/cast/src/config/approval.rs`
```rust
pub struct ApprovalStore {
    pub entries: BTreeMap<String, ApprovalEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApprovalEntry {
    pub workspace: String,
    pub approved_at: u64,
}
```

## 2. Limitations for Diffing

The current system has a significant limitation for implementing diffs: **it does not store the configuration content itself.**

- `ApprovalEntry` only stores the workspace path and the approval timestamp.
- The configuration is only represented by its hash.
- To show a diff, the system would need to store the last approved configuration in its entirety.

## 3. Configuration Display

The `cast config show` command currently only prints the JSON representation of the *current* configuration. It has no logic to look up previous versions or perform comparisons.

**Source:** `crates/cast/src/commands/config.rs`
```rust
pub fn handle_config(config: &Config, command: Option<ConfigCommands>) -> Result<ExitCode> {
    match command {
        Some(ConfigCommands::Show) | None => {
            let json = serde_json::to_string_pretty(&config)?;
            println!("{}", json);
            Ok(ExitCode::SUCCESS)
        }
        // ...
    }
}
```

## 4. Required Changes for Diff Support

To support the proposed "idea", the following architectural changes are necessary:

1.  **Schema Update**: Add a `config` field to `ApprovalEntry` to store the serialized `Config` (or the `Config` struct itself if it implements `Serialize`/`Deserialize`).
    ```rust
    pub struct ApprovalEntry {
        pub workspace: String,
        pub approved_at: u64,
        pub config: Config, // New field
    }
    ```
2.  **Storage Update**: Modify `ApprovalStore::add_entry` to include the current `Config` when saving a new approval.
3.  **Lookup Logic**: Add a method to `ApprovalStore` to retrieve the `ApprovalEntry` for a specific workspace path (since the store is currently indexed by hash, this would require iterating or adding a secondary index/mapping).
4.  **Diff Utility**: Integrate a diffing library (e.g., `similar` or `console`) to compare the `last_approved_config` with the `current_config`.
5.  **CLI Enhancement**: Update `cast config show` to:
    - Load the `ApprovalStore`.
    - Find the entry matching the current workspace.
    - If it exists and the current hash doesn't match, display a diff instead of (or in addition to) the full config.

## Summary of Findings

- **Current State**: The system only tracks hashes of approved configurations.
- **Blocker**: The actual content of the last approved configuration is discarded.
- **Path Forward**: The `ApprovalEntry` must be expanded to include the configuration content to enable meaningful diffs during the approval process.
