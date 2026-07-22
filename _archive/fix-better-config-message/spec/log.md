# Project Log

## [c871071] Implement context-aware config approval hints

Implemented the fix for misleading config messages on new projects. The key insight was that `is_approved → bool` lost the distinction between "never seen this workspace" vs "workspace known but config changed".

- **Found:** verify() was canonicalizing inside compute_config_hash but needed the canonical string separately for find_by_workspace — splitting the hash function cleanly solved this
- **Found:** cargo fmt reordered imports in 3 files (types before functions)
- **Decided:** Added ApprovalStatus enum with Approved/Changed/Unapproved variants
- **Decided:** Extracted compute_config_hash_canonical as private helper to avoid double-canonicalization in verify()
- **Decided:** check_status cross-checks workspace against hash entry to guard against collisions/hand-edited JSON (Flash suggestion)
- **Decided:** Used Unapproved over Absent/NeverApproved for clarity in match arms
- **Decided:** is_approved kept unchanged — still used in low-level unit tests

## [c370181] Refactor commands/config.rs to minimal entry point

Extracted all business logic from the Show and Diff command branches into the config domain module. The command handler is now a thin dispatcher that resolves workspace once, calls one domain function per branch, and handles presentation only.

- **Found:** Unit tests that called approve_workspace_config hit the real ~/.local/share/cast/approved_configs.json which was corrupt on this machine — drove the _with helper pattern
- **Decided:** Public functions get_approval_status/compute_workspace_diff delegate to private _with(store) helpers — keeps public API clean while making unit tests possible without global I/O
- **Decided:** store.entries layering violation (Diff branch directly indexing the map) eliminated by moving that logic into compute_workspace_diff_with
- **Decided:** Workspace resolved once at top of handle_config — Flash suggestion, gives early-fail and removes 2-line boilerplate from every branch

## [f04b0e9-dirty] Eliminate redundant canonicalize in approve_workspace_config

Two call sites were passing an already-canonical path into compute_config_hash, which unconditionally canonicalizes its argument before delegating to compute_config_hash_canonical. Swapped both to call compute_config_hash_canonical directly, eliminating the extra syscall. compute_config_hash_canonical is private but both sites are within approval.rs.

- **Found:** approve_workspace_config (line 221) and the test helper approve closure (line 659) both canonicalized the path once before passing it to compute_config_hash, which canonicalized it a second time internally
- **Found:** compute_config_hash_canonical is private (fn, not pub fn) but accessible from both sites since they are in the same file
- **Decided:** Call compute_config_hash_canonical directly at both sites instead of compute_config_hash to avoid the redundant fs::canonicalize syscall
- **Decided:** Did not touch any other call sites — those pass non-canonical paths and correctly rely on compute_config_hash to resolve them

