# Project Log

## [cdb2312-dirty] fix: unify config path — replace dirs::config_dir() with home_config_dir()

Implemented the fix-unify-config-path todo. Created xdg::home_config_dir() as a shared utility that always returns $HOME/.config regardless of platform. Replaced all 3 production call sites of dirs::config_dir() and aligned the opencode agent to use opts.host_home_dir (consistent with claudecode/pi). Committed on branch fix/unify-config-path.

- **Found:** dirs::config_dir() was called in 3 places: config/loader.rs (global config path), opencode/mod.rs prepare_host, opencode/mod.rs extra_run_args
- **Found:** claudecode and pi already derived config paths from opts.host_home_dir — opencode was the outlier calling dirs::config_dir() directly
- **Found:** One test (test_extra_run_args_workspace_conflict_no_double_mount) was also using dirs::config_dir() and needed fixing
- **Decided:** Created crates/cast/src/xdg.rs with home_config_dir() — always returns $HOME/.config
- **Decided:** Made opencode agent consistent with claudecode/pi: derive config base from opts.host_home_dir.join('.config') rather than calling dirs directly
- **Decided:** Staged only intentionally changed files to avoid committing pre-existing unrelated formatting diffs

## [9385740-dirty] refactor: rename xdg module to paths

Followed up on Sonnet's review finding that the module name 'xdg' was misleading (it doesn't implement XDG_CONFIG_HOME semantics). Renamed to 'paths' and added a doc comment explaining why XDG_CONFIG_HOME is intentionally ignored.

- **Decided:** Rename xdg → paths based on consultant-sonnet review feedback
- **Decided:** Added explicit doc note about XDG_CONFIG_HOME being intentionally ignored to prevent future confusion

