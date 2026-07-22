# Plan: Print Better Config Messages

## Ticket
`.mem/fix-better-config-message/todo/1781195741-852f18c/print-better-config-messages.md`

## Problem
On a new project, users see a confusing loop:
```
Run `cast config diff` to see what changed, then `cast config allow` to approve.
❯ cast config diff
No approved config for this workspace.
```
When there's no prior approval, `cast config diff` is useless. The hint should tell users to run `cast config allow` directly.

## Root Cause
Two locations emit a generic "run `cast config diff`" hint without checking whether there's actually a prior approval for the workspace:

1. `config/approval.rs` `verify()` — gates `run`, `build`, `shell`, etc.
2. `commands/config.rs` `Show` branch — stderr hint on `cast config show`

## Three Approval States

| State | `entries.contains_key(hash)` | `find_by_workspace()` | Meaning |
|---|---|---|---|
| `Approved` | true | Some | Hash matches → all good |
| `Changed` | false | Some | Prior approval exists but config has changed |
| `Unapproved` | false | None | No prior approval for this workspace |

## Changes

### 1. Add `ApprovalStatus` enum — `approval.rs`
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalStatus {
    /// Hash matches — config is approved for this workspace.
    Approved,
    /// Workspace has a prior approval, but the current hash doesn't match.
    Changed,
    /// No approval entry exists for this workspace at all.
    Unapproved,
}
```

### 2. Extract `compute_config_hash_canonical` — `approval.rs`
Split the existing `compute_config_hash` into:
- `fn compute_config_hash_canonical(config: &Config, canonical_root: &Path) -> Result<String>` — private, takes already-resolved path, does the actual hashing
- `pub fn compute_config_hash(config: &Config, workspace_root: &Path) -> Result<String>` — public wrapper that canonicalizes first, then delegates

This avoids double-canonicalization in `verify()`.

### 3. Add `check_status` — `ApprovalStore`
```rust
pub fn check_status(&self, hash: &str, canonical_workspace: &str) -> ApprovalStatus {
    // Cross-check workspace to guard against hash collisions / hand-edited JSON
    if let Some(entry) = self.entries.get(hash) {
        if entry.workspace == canonical_workspace {
            return ApprovalStatus::Approved;
        }
    }
    if self.find_by_workspace(canonical_workspace).is_some() {
        ApprovalStatus::Changed
    } else {
        ApprovalStatus::Unapproved
    }
}
```

### 4. Rewrite `verify()` — `approval.rs`
Canonicalize once, call `compute_config_hash_canonical`, then match on `check_status`:
```rust
pub fn verify(&self, config: Config, workspace_root: &Path) -> Result<ApprovedConfig> {
    let canonical = std::fs::canonicalize(workspace_root)
        .context("Failed to canonicalize workspace root path")?;
    let canonical_str = canonical.to_string_lossy();
    let hash = compute_config_hash_canonical(&config, &canonical)?;

    match self.check_status(&hash, &canonical_str) {
        ApprovalStatus::Approved => Ok(ApprovedConfig(config)),
        ApprovalStatus::Changed => anyhow::bail!(
            "Configuration has changed since last approval.\n\
             Note: env-var overrides (CAST_*) affect the hash.\n\
             Run `cast config diff` to see what changed, then `cast config allow` to approve."
        ),
        ApprovalStatus::Unapproved => anyhow::bail!(
            "Configuration has not been approved for this project.\n\
             Run `cast config allow` to approve the current configuration."
        ),
    }
}
```

### 5. Update `Show` hint — `commands/config.rs`
Replace the `is_approved` bool check with `check_status`:
```rust
let canonical = std::fs::canonicalize(&workspace.root)?;
let canonical_str = canonical.to_string_lossy();
let hash = compute_config_hash_canonical(&config, &canonical)?;  // or reuse existing hash
match store.check_status(&hash, &canonical_str) {
    ApprovalStatus::Approved => {}
    ApprovalStatus::Changed => eprintln!(
        "Note: config changed since last approval — run `cast config diff` to see what changed, or `cast config allow` to approve."
    ),
    ApprovalStatus::Unapproved => eprintln!(
        "Note: config not approved — run `cast config allow` to approve the current configuration."
    ),
}
```

## Tests

### Update existing tests
- `approval.rs` `test_verify_unapproved_config_fails` — update to check for new `Unapproved` message ("Configuration has not been approved")
- `approval.rs` `test_verify_approved_config` — no change needed (still passes)
- `config_test.rs` `test_config_show_hints_diff_when_unapproved` — currently asserts `cast config diff` in stderr; update to assert `cast config allow` (no prior approval case)

### Add new tests
- `approval.rs`: `test_check_status_approved` — hash+workspace match → `Approved`
- `approval.rs`: `test_check_status_changed` — workspace matches but hash differs → `Changed`
- `approval.rs`: `test_check_status_unapproved` — workspace not in store → `Unapproved`
- `approval.rs`: `test_check_status_hash_collision_guard` — hash matches but workspace differs → not `Approved` (falls through to `Changed` or `Unapproved`)
- `approval.rs`: `test_verify_changed_config_error_message` — verify `Changed` path produces the `config diff` hint
- `config_test.rs`: `test_config_show_hints_allow_when_unapproved` — no prior approval → stderr contains `cast config allow`, not `cast config diff`
- `config_test.rs`: `test_config_show_hints_diff_when_changed` — prior approval + config change → stderr contains `cast config diff`

## Files Changed
- `crates/cast/src/config/approval.rs`
- `crates/cast/src/commands/config.rs`
- `crates/cast/tests/config_test.rs`
