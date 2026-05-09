# cast config allow/deny — Technical Plan

## Architecture Overview

The feature is implemented across three layers:

1. **Core logic** (`src/config/approval.rs`): hash computation and the approval store.
2. **CLI commands** (`src/commands/config.rs`): `allow` and `deny` subcommands.
3. **Enforcement gate** (`src/dev/run.rs`): blocks unapproved runs.

---

## Layer 1: `src/config/approval.rs` (new file)

### Deterministic JSON Hashing

`Config` uses `BTreeMap` for map fields (`agent_versions` and `extra_data_volumes`), which guarantees deterministic iteration order during serialization. This allows for direct hashing of the serialized JSON bytes without an intermediate canonicalization step.

### `compute_config_hash`

The workspace root is canonicalized via `std::fs::canonicalize` before hashing.
This ensures that `~/project`, `/home/user/project`, and any symlink-based paths all
resolve to the same canonical form and produce identical hashes for the same workspace.

```rust
pub fn compute_config_hash(config: &Config, workspace_root: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::os::unix::ffi::OsStrExt;

    let canonical_root = std::fs::canonicalize(workspace_root)
        .context("Failed to canonicalize workspace root path")?;

    // Direct serialization is deterministic because of BTreeMap in Config.
    let config_bytes = serde_json::to_vec(config)?;

    let mut hasher = Sha256::new();
    hasher.update(canonical_root.as_os_str().as_bytes());
    hasher.update(b"\0"); // null separator prevents path/config boundary collisions
    hasher.update(&config_bytes);

    Ok(hex::encode(hasher.finalize()))
}
```

### Typestate Pattern: `ApprovedConfig`

To ensure the security gate cannot be bypassed, we use the Typestate Pattern.

```rust
/// A typestate representing a Configuration that has passed the security approval gate.
#[derive(Debug, Clone)]
pub struct ApprovedConfig(Config);

impl ApprovedConfig {
    pub fn into_inner(self) -> Config { self.0 }

    #[cfg(test)]
    pub fn assume_approved_for_test(config: Config) -> Self { Self(config) }
}

impl Deref for ApprovedConfig {
    type Target = Config;
    fn deref(&self) -> &Self::Target { &self.0 }
}
```

The only production-ready way to obtain an `ApprovedConfig` is via `ApprovalStore::verify`:

```rust
impl ApprovalStore {
    pub fn verify(&self, config: Config, workspace_root: &Path) -> Result<ApprovedConfig> {
        let hash = compute_config_hash(&config, workspace_root)?;
        if self.is_approved(&hash) {
            Ok(ApprovedConfig(config))
        } else {
            anyhow::bail!("Configuration has not been approved for this project...");
        }
    }
}
```

### Approval Store

`BTreeMap` is used for the entries collection to ensure the persisted JSON file always has lexicographically sorted keys.

```rust
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ApprovalStore {
    pub entries: BTreeMap<String, ApprovalEntry>,
}
```

**Persistence path**: `dirs::data_dir().join("cast").join("approved_configs.json")`
**Atomic writes**: Use `NamedTempFile` + `persist()`.
**File permissions**: Parent directory `0o700`, file `0o600`.

---

## Layer 2: `src/commands/config.rs` (updated)

`cast config allow/deny/show` operate on the raw `Config`. `allow` and `deny` are silent on success. `show` only prints the configuration JSON, without computing hashes, ensuring it works outside of workspace contexts.

---

## Layer 3: Enforcement & Dispatcher

### Interception Point (`src/commands/cli.rs`)

The approval check is lifted out of `run_agent` and into the command dispatcher.

```rust
fn verify_config(cfg: Config) -> Result<ApprovedConfig> {
    let user = get_user()?;
    let workspace = get_workspace(&user.username)?;
    let store = load_approval_store()?;
    store.verify(cfg, &workspace.root)
}

// In run():
Some(Commands::Run { agent }) => {
    let approved_cfg = verify_config(cfg)?;
    dev::run_agent(agent.as_agent(), &approved_cfg, ...)?;
}
```

### Enforcement Gate (`src/dev/run.rs`)

`run_agent` (and `shell`) change their signature to accept `&ApprovedConfig`. This makes it a compile-time error to call these functions without having passed through the verification helper.

---

## Testing Strategy

### `src/config/approval.rs` (unit tests)
- `compute_config_hash` stability and sensitivity tests.
- `test_config_hash_determinism`: Regression guard for insertion order.
- `ApprovalStore::verify` success and failure cases.
- `save` + `load` round-trips with permissions checks.

### `src/dev/run.rs` & `src/dev/shell.rs`
- Unit tests calling these functions must use `ApprovedConfig::assume_approved_for_test`.

