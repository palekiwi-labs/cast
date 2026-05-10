# Code Review: `cast config allow` Overwrite Behavior

**Consultant**: Gemini
**Date**: Sun May 10 2026

## Summary
The implementation correctly centralizes the single-entry-per-workspace invariant in `ApprovalStore::add_entry`. However, a critical edge case exists regarding path canonicalization that could allow stale approvals to persist if a workspace is accessed via different paths (e.g., symlinks).

## Key Findings

### 1. Architectural Choice (add_entry)
- **Positive**: Moving the cleanup logic into `ApprovalStore::add_entry` is praised for centralizing data integrity at the domain layer.

### 2. Path Canonicalization Mismatch (Critical)
- **Issue**: `compute_config_hash` canonicalizes the path, but `approve_workspace_config` and `deny_workspace_config` use the raw `to_string_lossy()` of the path provided by the CLI.
- **Impact**: String-based matching in `remove_workspace_entries` will fail to identify and remove existing entries if they were added via a different path representation (like a symlink).
- **Recommendation**: Canonicalize the path string at the entry points (`approve_workspace_config` and `deny_workspace_config`) to ensure the store only deals with consistent canonical paths.

### 3. Test Coverage
- **Suggestion**: Add integration-level tests that exercise `approve_workspace_config` and `deny_workspace_config` directly, rather than just the in-memory store.
- **Suggestion**: Test path resolution edge cases (symlinks vs. real paths).

## Action Plan
1. Update `approve_workspace_config` and `deny_workspace_config` in `src/config/approval.rs` to canonicalize the workspace path before passing it to the store.
2. Refactor `compute_config_hash` to avoid redundant canonicalization if possible, or ensure it uses the same canonicalized path.
3. Add a test case for symlink-based path matching.