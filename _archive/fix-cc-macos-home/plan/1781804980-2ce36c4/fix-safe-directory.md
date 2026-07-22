---
status: open
---
# Plan: Fix git safe.directory in Agent Dockerfiles

Ref task: `task/1781804980-2ce36c4/bug-fix-claudecode-agent-on-macos.md`
Ref trace: `trace/1781804980-2ce36c4/safe-directory-bug-analysis.md`

## Goal

Add `RUN git config --system safe.directory "*"` to all three agent Dockerfiles
and remove the unnecessary git config from opencode and pi, standardizing all three
to be as minimal as claudecode was (before this fix).

## Affected Files

| File | Change |
|------|--------|
| `crates/cast/assets/Dockerfile.dev.claudecode` | Add `RUN git config --system safe.directory "*"` |
| `crates/cast/assets/Dockerfile.dev.opencode` | Replace git config block with `RUN git config --system safe.directory "*"` |
| `crates/cast/assets/Dockerfile.dev.pi` | Replace git config block with `RUN git config --system safe.directory "*"` |
| `crates/cast/src/dev/claudecode/mod.rs` | Add Dockerfile assertion test |
| `crates/cast/src/dev/opencode/mod.rs` | Add Dockerfile assertion test, update/remove outdated test |
| `crates/cast/src/dev/pi/mod.rs` | Add Dockerfile assertion test, update/remove outdated test |

## TDD Cycles (vertical slices)

### Cycle 1 — ClaudeCode (tracer bullet)

**RED**: Add test to `claudecode/mod.rs`:
```rust
#[test]
fn test_dockerfile_configures_git_safe_directory() {
    assert!(ClaudeCode.dockerfile()
        .contains(r#"git config --system safe.directory "*""#));
}
```

**GREEN**: Add to `Dockerfile.dev.claudecode` (before the user-creation block):
```dockerfile
# Allow git operations in bind-mounted workspaces owned by different users.
# Required for `nix develop .` (libgit2) on macOS/Docker Desktop and Linux
# with userns-remap. Mirrors Dockerfile.nix-daemon.
RUN git config --system safe.directory "*"
```

**COMMIT**: `fix: configure git safe.directory in claudecode Dockerfile`

---

### Cycle 2 — OpenCode

**RED**: Add test to `opencode/mod.rs`:
```rust
#[test]
fn test_dockerfile_configures_git_safe_directory() {
    assert!(OpenCode.dockerfile()
        .contains(r#"git config --system safe.directory "*""#));
}
```

**GREEN**: Replace the entire git config `RUN` block in `Dockerfile.dev.opencode`
with the single `safe.directory` line.

**RED** (regression guard): Remove or update any test that asserted the now-deleted
git config lines (e.g. `user.name`, `init.defaultBranch`).

**COMMIT**: `fix: standardize opencode Dockerfile git config to safe.directory only`

---

### Cycle 3 — Pi

Same pattern as Cycle 2.

**COMMIT**: `fix: standardize pi Dockerfile git config to safe.directory only`

---

## Invariants

- All three Dockerfiles end up structurally identical in their git config section
- No extra git settings (`user.name`, `user.email`, etc.) remain in any agent Dockerfile
- `--system` scope is used (consistent with other system-wide config; `--global` is
  for root's home dir and is used only by the nix-daemon which always runs as root)
- Tests assert the Dockerfile content — same pattern as existing
  `test_dockerfile_has_correct_base_image` tests
