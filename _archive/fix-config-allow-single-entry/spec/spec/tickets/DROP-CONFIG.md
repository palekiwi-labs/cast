# Task: Drop `opencode_config` Special Handling

## Goal
Simplify agent logic and improve transparency by removing specialized mounting for `OPENCODE_CONFIG` and `OPENCODE_CONFIG_DIR`.

## Steps
1.  **Remove specialized mounting logic** in `src/dev/opencode/mod.rs` (functions `resolve_config_dir_env`, `resolve_config_file_env`, and their usage in `extra_run_args`).
2.  **Update environment passthrough** in `src/dev/opencode/env.rs`.
3.  **Delete associated tests** in `src/dev/opencode/mod.rs`.
4.  **Verify** that `extra_data_volumes` in `cast.json` and `cast.env` can successfully replace the dropped functionality.

## Acceptance Criteria
- `cargo test` passes.
- Codebase is free of `OPENCODE_CONFIG_DIR` special-case mounting logic.
- Documentation/comments correctly reflect that users must manually mount custom config files/dirs.
