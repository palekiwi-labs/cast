# Project Log

## [99cc22f] Research complete: Configuration split feasibility

Researched the feasibility of splitting cast configuration into cast.json and cast-mcp.json. Confirmed that figment supports recursive merging and optional files, and that the existing security model (config approval) will naturally extend to the new file if it's merged into the same Config struct.

- **Found:** Figment supports recursive union of nested structures.
- **Found:** Figment handles missing files gracefully as empty providers.
- **Found:** Config approval is based on the final resolved Config struct, meaning cast-mcp.json changes will be caught.
- **Decided:** Split configuration can be implemented by adding cast-mcp.json to the figment merge chain in loader.rs.

## [1a4efa1] Implemented configuration split into cast.json and cast-mcp.json

Implemented configuration split into cast.json and cast-mcp.json.
- Updated `load_config` in `loader.rs` to merge `cast-mcp.json` after `cast.json`.
- Added unit tests to verify merging behavior, precedence, and handling of missing files.
- Updated MCP configuration documentation to reflect the new file support.
- Applied formatting and clippy fixes.
- Verified that the security model (config approval) naturally supports the new file since it's merged into the same Config struct before hashing.

- **Found:** Figment's recursive union correctly merges tools from both files.
- **Decided:** cast-mcp.json has higher precedence than cast.json but lower than environment variables.

## [1a4efa1] Research Start: Env Var List Loading

Starting research into environment variable list loading in `cast` to explain a figment error.

## [0e8bd95] Migrated MCP config and cleaned up test files

Migrated existing MCP configuration from `cast.json` to the newly supported `cast-mcp.json` file. Cleaned up temporary test files used during implementation verification.

- **Decided:** Moved repo-level MCP settings to cast-mcp.json.

## [05a85c3-dirty] Implemented flat structure support for cast-mcp.json

Implemented flat structure support for `cast-mcp.json`.
- Modified `loader.rs` to extract `cast-mcp.json` into an intermediate `Value` and then merge it using `Serialized::defaults(mcp_json).key("mcp")`.
- This allows `cast-mcp.json` to NOT require a root `mcp` key, while still correctly populating the `mcp` field of the `Config` struct.
- Updated unit tests to verify the flat structure merging and precedence.
- Updated documentation with an example of the flat `cast-mcp.json` structure and an explanation of the requirement.
- Verified that missing `cast-mcp.json` is still handled gracefully.

- **Found:** Figment's Serialized::key(path) allows for precise targeting of merged data.
- **Decided:** cast-mcp.json now uses a flat structure (no root mcp key).
- **Decided:** Explicitly nesting the loaded value under the mcp key during merge.

## [43e28ff] Refactored config loader to eliminate CWD mutation in tests

Introduced `load_config_from(base_dir: &Path)` as the core loading function and made `load_config()` a thin wrapper around it. Removed the `CWD_MUTEX` and all `std::env::set_current_dir` calls from unit tests — tests now pass the tempdir path directly. Also fixed the latent race condition in `tests/config_test.rs` where `test_config_load_with_partial_json` was calling `set_current_dir` without any lock. Exported `load_config_from` from `config/mod.rs`.

- **Found:** Integration test config_test.rs had a latent race condition using set_current_dir without a mutex.
- **Decided:** load_config_from is the authoritative loader; load_config() is a CWD-delegating wrapper.

