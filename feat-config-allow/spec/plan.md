# cast config allow/deny — Technical Plan

## Architecture Overview

The feature is implemented across three layers:

1. **Core logic** (`src/config/approval.rs`): hash computation and the approval store.
2. **CLI commands** (`src/commands/config.rs`): `allow` and `deny` subcommands.
3. **Enforcement gate** (`src/dev/run.rs`): blocks unapproved runs.

---

## Layer 1: `src/config/approval.rs` (new file)

### Canonical JSON Hashing

`Config` contains `HashMap<String, String>` and `HashMap<String, VolumeConfig>` whose
serialization order is non-deterministic. A canonicalization step is required before hashing.

**Approach**: Serialize `Config` to `serde_json::Value`, recursively sort all object
keys alphabetically, serialize the sorted `Value` to bytes, then hash with SHA-256.

```rust
/// Recursively sort all JSON object keys for deterministic serialization.
fn canonicalize_value(v: serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(map) => {
            let sorted: serde_json::Map<_, _> = map
                .into_iter()
                .map(|(k, v)| (k, canonicalize_value(v)))
                .collect::<std::collections::BTreeMap<_, _>>()
                .into_iter()
                .collect();
            serde_json::Value::Object(sorted)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(canonicalize_value).collect())
        }
        other => other,
    }
}
```

### `compute_config_hash`

The workspace root is canonicalized via `std::fs::canonicalize` before hashing.
This ensures that `~/project`, `/home/user/project`, and any symlink-based paths all
resolve to the same canonical form and produce identical hashes for the same workspace.

```rust
pub fn compute_config_hash(config: &Config, workspace_root: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::os::unix::ffi::OsStrExt; // required for .as_bytes() on OsStr

    let canonical_root = std::fs::canonicalize(workspace_root)
        .context("Failed to canonicalize workspace root path")?;

    let json_value = serde_json::to_value(config)?;
    let canonical_json = canonicalize_value(json_value);
    let config_bytes = serde_json::to_vec(&canonical_json)?;

    let mut hasher = Sha256::new();
    hasher.update(canonical_root.as_os_str().as_bytes()); // OsStrExt::as_bytes
    hasher.update(b"\0"); // null separator prevents path/config boundary collisions
    hasher.update(&config_bytes);

    Ok(hex::encode(hasher.finalize()))
}
```

Note: `OsStr::as_encoded_bytes()` does not exist in stable Rust. The correct method
on Unix targets is `std::os::unix::ffi::OsStrExt::as_bytes()`.

### Approval Store

`BTreeMap` is used instead of `HashMap` for the entries collection. This ensures
the persisted JSON file always has lexicographically sorted keys, making diffs and
debugging readable and the serialized form deterministic.

```rust
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ApprovalStore {
    pub entries: BTreeMap<String, ApprovalEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApprovalEntry {
    pub workspace: String,
    pub approved_at: u64, // seconds since Unix epoch
}
```

**Persistence path**: `dirs::data_dir().join("cast").join("approved_configs.json")`

**Atomic writes**: Use the same `NamedTempFile` + `persist()` pattern as `src/dev/version/cache.rs`.

**File permissions**: The parent directory must be created with mode `0o700` and the
file with mode `0o600` (owner-only). This prevents other users on a shared Linux
machine from reading or modifying the approval store. Use
`std::os::unix::fs::DirBuilderExt` and `OpenOptionsExt` when creating.

**`load_approval_store` error handling**: `NotFound` must be handled explicitly and
return `ApprovalStore::default()`. All other I/O errors must be propagated. Silently
swallowing non-`NotFound` errors would mask a corrupted store.

```rust
pub fn load_approval_store() -> Result<ApprovalStore> {
    let path = approval_store_path();
    match std::fs::read_to_string(&path) {
        Ok(raw) => Ok(serde_json::from_str(&raw)
            .context("Failed to parse approval store — file may be corrupted")?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ApprovalStore::default()),
        Err(e) => Err(e).context("Failed to read approval store"),
    }
}
```

**Public API**:
```rust
pub fn approval_store_path() -> PathBuf { ... }
pub fn load_approval_store() -> Result<ApprovalStore> { ... }

impl ApprovalStore {
    pub fn is_approved(&self, hash: &str) -> bool { ... }
    pub fn add_entry(&mut self, hash: String, workspace: String) { ... }
    pub fn remove_entry(&mut self, hash: &str) { ... }
    pub fn save(&self) -> Result<()> { ... } // atomic write with 0o600 file permissions
}
```

---

## Layer 2: `src/commands/config.rs` (updated)

### New subcommands

```rust
#[derive(clap::Subcommand)]
pub enum ConfigCommands {
    /// Show the current configuration
    Show,
    /// Approve the current configuration for this project
    Allow,
    /// Revoke approval for the current configuration in this project
    Deny,
}
```

### Handler

Both `Allow` and `Deny` require resolving the current user and workspace (for the
workspace path to include in the hash). The same pattern used in `run_agent` applies.

```rust
ConfigCommands::Allow => {
    let user = get_user()?;
    let workspace = get_workspace(&user.username)?;
    let hash = compute_config_hash(config, &workspace.root)?;
    let mut store = load_approval_store()?;
    store.add_entry(hash.clone(), workspace.root.to_string_lossy().into_owned());
    store.save()?;
    println!("Configuration approved ({}).", &hash[..12]);
    Ok(ExitCode::SUCCESS)
}

ConfigCommands::Deny => {
    let user = get_user()?;
    let workspace = get_workspace(&user.username)?;
    let hash = compute_config_hash(config, &workspace.root)?;
    let mut store = load_approval_store()?;
    store.remove_entry(&hash);
    store.save()?;
    println!("Approval revoked ({}).", &hash[..12]);
    Ok(ExitCode::SUCCESS)
}
```

---

## Layer 3: `src/dev/run.rs` (updated)

### Interception point

The approval check occurs in `run_agent` after the workspace is resolved and before
`nix_daemon::ensure_running`, which is the first side-effecting operation.

The rejection message includes the short hash so the user can correlate it with what
`cast config allow` will approve. It also mentions env-var overrides to prevent
confusion when the hash appears to change unexpectedly.

```rust
pub fn run_agent(agent: &dyn Agent, config: &Config, extra_args: Vec<String>) -> Result<ExitStatus> {
    let start_time = Instant::now();
    let docker = DockerClient;
    let user = get_user()?;
    let workspace = get_workspace(&user.username)?;

    // Config approval gate — must precede all side effects.
    let hash = compute_config_hash(config, &workspace.root)?;
    let store = load_approval_store()?;
    if !store.is_approved(&hash) {
        anyhow::bail!(
            "Configuration has not been approved for this project (hash: {}).\n\
             Note: env-var overrides (CAST_*) affect the hash.\n\
             Review with `cast config show`, then run `cast config allow` to approve.",
            &hash[..8]
        );
    }

    // ... rest of run_agent unchanged ...
    nix_daemon::ensure_running(&docker, config)?;
```

---

## Module Wiring

`src/config/mod.rs` exposes the new module:
```rust
mod approval;
pub use approval::{compute_config_hash, load_approval_store, ApprovalStore, ApprovalEntry};
```

---

## Testing Strategy

### `src/config/approval.rs` (unit tests)
- `compute_config_hash` produces the same result for the same config + workspace (stability).
- `compute_config_hash` produces different results for different workspace paths (use `tempfile::TempDir`).
- `compute_config_hash` produces different results for different configs.
- `compute_config_hash` is stable regardless of `HashMap` insertion order for `extra_data_volumes`.
- `ApprovalStore::add_entry` → `is_approved` returns true.
- `ApprovalStore::remove_entry` → `is_approved` returns false.
- `load_approval_store` returns an empty store when file is absent.
- `load_approval_store` propagates error on corrupted (non-JSON) file.
- `save` + `load_approval_store` round-trips correctly.

### `src/commands/config.rs`
- Existing `Show` test coverage is maintained.

### `src/dev/run.rs`
- `run_agent` can be tested only at the integration level (requires Docker). No new
  unit tests are needed here; the approval logic itself is unit-tested in `approval.rs`.

---

## Error Messages

| Situation | Message |
|---|---|
| Config not approved | `"Configuration has not been approved for this project (hash: abc12345).\nNote: env-var overrides (CAST_*) affect the hash.\nReview with \`cast config show\`, then run \`cast config allow\` to approve."` |
| After `cast config allow` | `"Configuration approved (abc12345678abc)."` |
| After `cast config deny` | `"Approval revoked (abc12345678abc)."` |
| `cast config deny` on non-approved config | `"Approval revoked (abc12345678abc)."` (idempotent — `remove_entry` is a no-op if the key is absent) |

## Enhancement: `cast config show` Displays Hash and Approval Status

`cast config show` must be updated to also compute and display the current hash and
its approval status. This gives users full visibility before deciding to approve.

```
$ cast config show
{
  "memory": "1024m",
  ...
}

Hash:   abc12345678abcde...
Status: NOT APPROVED — run `cast config allow` to approve
```

This requires `show` to also call `get_user()` + `get_workspace()` for hash
computation. The approval status lookup is read-only and does not modify the store.
