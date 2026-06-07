# Plan: Config Diff Support

## Goal

Show a colored diff between the last approved configuration and the current state
via `cast config diff`, so users clearly see what changed before re-approving.

## Architecture Overview

### 1. Schema extension (`config/approval.rs`)

Extend `ApprovalEntry` with an `approved_config` field that stores the full
configuration as a `serde_json::Value` snapshot at the time of approval.
`#[serde(default)]` ensures existing store files load cleanly without migration —
old entries simply deserialize to `None`.

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct ApprovalEntry {
    pub workspace:       String,
    pub approved_at:     u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_config: Option<serde_json::Value>,  // new
}
```

The snapshot is created from the same `serde_json::to_value(config)` call used
during hashing, guaranteeing the stored blob is byte-consistent with what the
hash was computed over.

`add_entry` is updated to accept the snapshot, and `approve_workspace_config`
captures and passes it through.

A new `find_by_workspace(canonical_path: &str) -> Option<&ApprovalEntry>` method
is added to `ApprovalStore` to allow workspace-based lookup (the store is
currently indexed by hash, not workspace).

### 2. New diff module (`config/diff.rs`)

A focused module that takes two `serde_json::Value` snapshots and returns a
formatted string containing a colored unified diff.

- Uses `similar::TextDiff::from_lines` on the pretty-printed JSON of each value.
- Uses `similar::unified_diff` with `context_radius(3)` so only changed regions
  (plus 3 lines of surrounding context) are shown — consistent with `git diff`.
- Uses `owo-colors` with a terminal-awareness check (`std::io::IsTerminal`) so
  ANSI codes are suppressed when output is piped.
- Colors: red for removed lines (`-`), green for added lines (`+`), dim for
  context lines (` `).

### 3. New `cast config diff` subcommand (`commands/config.rs`)

A first-class subcommand. `cast config show` is left unchanged (its stdout
contract is raw JSON, which may be piped to `jq` etc.).

Handler logic, in order:

1. Load the `ApprovalStore` and get the canonical workspace path.
2. Call `store.find_by_workspace(...)`.
3. **No entry found** → print: "No approved config for this workspace. Run
   `cast config allow` to approve."
4. **Entry found, `approved_config` is `None`** → print: "No snapshot available
   for this workspace (approved with an older version of cast). Run
   `cast config allow` to re-approve and capture a snapshot."
5. **Entry found, snapshot present**:
   - Compute the current config hash.
   - If hash matches the store entry key → print: "Config matches approved
     state. No changes."
   - If hash differs → print the formatted diff via `format_config_diff`.

### 4. Passive stderr hint in `cast config show`

When the current config is not approved, emit a single line to **stderr** (not
stdout, to preserve piping):

```
Note: config not approved — run `cast config diff` to see what changed.
```

This requires loading the store and computing the current hash inside the `Show`
handler.

### 5. Update verification failure message

In `ApprovalStore::verify`, update the `anyhow::bail!` message to mention
`cast config diff` alongside `cast config show`.

## Why a plain text diff is sufficient

The `Config` schema uses `BTreeMap` for all named maps (`extra_data_volumes`,
`agent_versions`, `mcp.tools`, `McpEnvConfig.set`), which guarantees
alphabetically sorted and therefore deterministic key output in
`serde_json::to_string_pretty`. Vec fields preserve source order from the config
file. `serde_json::Value` fields in `McpToolConfig.parameters` preserve insertion
order via IndexMap. In practice, the serialized output of the same config file is
identical across runs, making a line-based diff fully correct and readable.

## New dependencies

```toml
similar   = "2"
owo-colors = "4"
```

## Affected files

| File | Change |
|---|---|
| `crates/cast/Cargo.toml` | Add `similar`, `owo-colors` |
| `crates/cast/src/config/approval.rs` | Extend `ApprovalEntry`; update `add_entry` and `approve_workspace_config`; add `find_by_workspace` |
| `crates/cast/src/config/diff.rs` | New — `format_config_diff` utility |
| `crates/cast/src/config/mod.rs` | Expose `diff` module and `format_config_diff` |
| `crates/cast/src/commands/config.rs` | Add `Diff` variant; new `Diff` handler; update `Show` handler |
