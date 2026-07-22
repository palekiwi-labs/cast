# Todo List: `cast config allow` Overwrite

- [x] Create a reproduction test case to confirm multiple entries can currently exist for one workspace. <!-- id: 0 -->
- [x] Modify `approve_workspace_config` in `src/config/approval.rs` to call `remove_workspace_entries` before `add_entry`. <!-- id: 1 -->
- [x] Run the reproduction test case to verify it now enforces a single entry. <!-- id: 2 -->
- [x] Verify `cast config deny` still works as expected. <!-- id: 3 -->
- [x] Fix path canonicalization in `approve_workspace_config` and `deny_workspace_config` to handle symlinks. <!-- id: 4 -->
- [x] Add integration test for symlink path matching in `src/config/approval.rs`. <!-- id: 5 -->
