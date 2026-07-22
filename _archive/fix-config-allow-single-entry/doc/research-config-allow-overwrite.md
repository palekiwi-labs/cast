# Research Report: `cast config allow` overwrite behavior

This report documents the current state of configuration approval persistence and the necessary changes to ensure exactly one approved version per workspace.

## Research Questions Answered
1. How are configuration approvals currently persisted?
2. Why does the current implementation allow multiple approved versions per workspace?
3. What is the current behavior of `cast config deny`?

## 1. Approval Persistence Structure

Approvals are stored in a `BTreeMap` where the key is a SHA256 hash of the configuration and the workspace path.

**Source:** `/home/pl/code/palekiwi-labs/cast/src/config/approval.rs`
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

## 2. Multi-Entry Behavior in `allow`

The `approve_workspace_config` function currently appends to the store. Because the `BTreeMap` key is the hash, changing the configuration results in a new hash and thus a new entry, while the old entry remains valid for that workspace.

**Source:** `/home/pl/code/palekiwi-labs/cast/src/config/approval.rs`
```rust
pub fn approve_workspace_config(config: &Config, workspace_root: &Path) -> Result<()> {
    let hash = compute_config_hash(config, workspace_root)?;
    let mut store = load_approval_store()?;
    store.add_entry(hash, workspace_root.to_string_lossy().into_owned());
    store.save()
}
```

The `add_entry` method simply inserts into the map:
```rust
pub fn add_entry(&mut self, hash: String, workspace: String) {
    self.entries.insert(
        hash,
        ApprovalEntry {
            workspace,
            approved_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        },
    );
}
```

## 3. `deny` Behavior

The `deny_workspace_config` function correctly removes all entries for a given workspace path.

**Source:** `/home/pl/code/palekiwi-labs/cast/src/config/approval.rs`
```rust
pub fn deny_workspace_config(workspace_root: &Path) -> Result<()> {
    let mut store = load_approval_store()?;
    store.remove_workspace_entries(&workspace_root.to_string_lossy());
    store.save()
}
```

The `remove_workspace_entries` method uses `retain` to filter out all entries matching the workspace path:
```rust
pub fn remove_workspace_entries(&mut self, workspace_path: &str) {
    self.entries
        .retain(|_, entry| entry.workspace != workspace_path);
}
```

## Summary of Findings

- **Current State**: `cast config allow` appends new hashes to the global approval store without removing old ones for the same workspace. This allows a workspace to have multiple "approved" configuration states simultaneously if they were approved at different times.
- **Desired State**: `cast config allow` should call `remove_workspace_entries` before `add_entry` to ensure only the most recent approval is kept.
- **`cast config deny`**: Already correctly removes all entries for the workspace. While it could theoretically be simplified if we guaranteed only one entry, keeping it as-is is safer to handle any legacy or inconsistent state.
