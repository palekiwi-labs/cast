# Project Log

## [cba5b96] Dropped opencode_config special handling

- **Found:** Removed resolve_config_dir_env and resolve_config_file_env
- **Found:** Cleaned up extra_run_args in src/dev/opencode/mod.rs
- **Found:** Removed associated tests
- **Decided:** Simplified agent logic in favor of explicit user configuration

## [768752c] Cleanup of stale comments and docs

- **Found:** Removed stale doc-comment in env.rs
- **Found:** Cleaned up outdated test comments in approval.rs
- **Decided:** Refined code and documentation after peer review

## [1f6b59c] Added OPENCODE_CONFIG_DIR to passthrough

- **Found:** Included OPENCODE_CONFIG_DIR in PASSTHROUGH_VARS
- **Decided:** Allow manual environment variable configuration for custom config directories

