# Implementation Plan: `cast config allow` Overwrite

We will modify the approval logic in `src/config/approval.rs` to enforce a "single approval per workspace" invariant.

## Proposed Changes

### 1. Modify `approve_workspace_config`
In `src/config/approval.rs`, update the `approve_workspace_config` function. Currently, it calls `store.add_entry`. We will update it to call `store.remove_workspace_entries` immediately before `add_entry`.

**Target File**: `src/config/approval.rs`
```rust
pub fn approve_workspace_config(config: &Config, workspace_root: &Path) -> Result<()> {
    let hash = compute_config_hash(config, workspace_root)?;
    let mut store = load_approval_store()?;
    
    // New step: clear existing entries for this workspace to ensure overwrite behavior
    store.remove_workspace_entries(&workspace_root.to_string_lossy());
    
    store.add_entry(hash, workspace_root.to_string_lossy().into_owned());
    store.save()
}
```

### 2. Validation
We will add a regression test to verify that calling `allow` multiple times with different configurations result in only the latest one being present in the store.

## Considerations
- **Atomicity**: The `save()` method already uses `tempfile` for atomic writes, so the "remove then add" sequence is safe as long as it happens within the same `store` instance before `save()`.
- **`cast config deny`**: No changes are strictly necessary as it already removes all entries, which aligns with the "assume other entries might exist" philosophy.
