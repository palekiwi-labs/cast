# Plan: Refactor `commands/config.rs` to Minimal Entry Point

## Goal
`src/commands/` files are minimal entry points: parse CLI args, call domain logic, print results, return `ExitCode`. Business logic lives in domain modules.

`commands/config.rs` currently violates this — the `Show` and `Diff` branches contain multi-step orchestration and a layering violation (direct `store.entries` access in `Diff`).

## Problems to Fix

| Branch | Violation |
|---|---|
| `Show` | 6-step inline orchestration: user → workspace → store → canonicalize → hash → check_status |
| `Diff` | Same orchestration + direct `store.entries.get(&hash)` access from a command handler |
| `Allow` | Already thin ✓ |
| `Deny` | Already thin ✓ |

## Changes

### 1. Add to `config/approval.rs`

**`get_approval_status`** — wraps the Show orchestration:
```rust
pub fn get_approval_status(config: &Config, workspace_root: &Path) -> Result<ApprovalStatus>
```
Internally: loads store, canonicalizes (using `compute_config_hash_canonical`), calls `check_status`.

**`ConfigDiffOutput`** + **`compute_workspace_diff`** — wraps the Diff orchestration:
```rust
pub enum ConfigDiffOutput {
    Unapproved,
    Unchanged,
    Changed(String), // plain diff string, no ANSI
}

pub fn compute_workspace_diff(config: &Config, workspace_root: &Path) -> Result<ConfigDiffOutput>
```
Internally: loads store, finds entry by workspace, computes hash with `compute_config_hash_canonical`, compares, generates diff via `format_config_diff`. Returns `Unchanged` for both "hash matches" and "diff string is empty" cases.

Both live in `approval.rs` (not a new module) — it already holds `approve_workspace_config` and `deny_workspace_config` as higher-level orchestrations. `diff.rs` stays a pure text-formatting module with no store coupling.

Export both from `config/mod.rs`.

### 2. Rewrite `commands/config.rs`

Resolve workspace **once** at the top of `handle_config` — all 4 branches need it, and this gives an early fail on environment/workspace errors:

```rust
pub fn handle_config(config: &Config, command: Option<ConfigCommands>) -> Result<ExitCode> {
    let user = get_user()?;
    let workspace = get_workspace(&user.username)?;

    match command {
        Show   => print JSON + call get_approval_status() + print hint to stderr
        Allow  => approve_workspace_config(config, &workspace.root)?
        Deny   => deny_workspace_config(&workspace.root)?
        Diff   => compute_workspace_diff(config, &workspace.root)? + print with colors
    }
}
```

The color rendering loop (terminal detection + ANSI via `owo_colors`) stays in the command — pure presentation concern. `format_config_diff` already returns a plain string.

After refactor, `commands/config.rs` imports no `load_approval_store`, no `compute_config_hash`, no `ApprovalStatus`, no `format_config_diff`, no `std::fs::canonicalize`. It only calls the two new high-level functions.

### 3. Add unit tests to `config/approval.rs`

New domain functions are easily testable with `tempfile::TempDir`, no CLI layer needed:
- `test_get_approval_status_approved`
- `test_get_approval_status_changed`
- `test_get_approval_status_unapproved`
- `test_compute_workspace_diff_unapproved`
- `test_compute_workspace_diff_unchanged`
- `test_compute_workspace_diff_changed`

Existing integration tests in `config_test.rs` provide end-to-end coverage of the command layer and don't need changes.

## Files Changed
- `crates/cast/src/config/approval.rs` — add `ConfigDiffOutput`, `get_approval_status`, `compute_workspace_diff` + tests
- `crates/cast/src/config/mod.rs` — export new types/functions
- `crates/cast/src/commands/config.rs` — rewrite to minimal entry point
