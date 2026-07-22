## Status: Complete

# Executive Plan: cast-mcp-client structural refactor

## Foreword

This plan implements the structural refactor of `crates/cast-mcp-client/src/`
described in `plan/index.md`. It covers all seven phases in a single execution
pass. The approach is purely mechanical: no logic changes, no API changes.

The key constraint is that `cargo test -p cast-mcp-client` must pass after
every phase so we can catch import breakage early.

Prerequisites:
- Working tree is on branch `feat/refactor-mcp-client`
- All existing tests currently pass

---

## Steps

### Phase 1 — Split `config.rs` into `config/`

- [x] 1.1 Create `src/config/schema.rs` — move `ClientConfig`, `RemoteServerConfig`,
      `default_enabled` from `config.rs`
- [x] 1.2 Create `src/config/loader.rs` — move `parse_from_str`, `load`,
      `load_from_files`, `merge`, `global_config_path`, `load_single_file`,
      `apply_env_substitution` + all `#[cfg(test)]` from `config.rs`
- [x] 1.3 Create `src/config/mod.rs` — re-export everything from schema + loader
- [x] 1.4 Delete `src/config.rs`
- [x] 1.5 Update `lib.rs`: `pub mod config;` still works (no change needed)
- [x] 1.6 `cargo test -p cast-mcp-client` — must pass

### Phase 2 — Create `client/`

- [x] 2.1 Create `src/client/handler.rs` — move `McpClientHandler` + its
      `ClientHandler` impl from `lib.rs`
- [x] 2.2 Create `src/client/mcp_client.rs` — move `McpClient` struct and all
      its `impl McpClient` methods from `lib.rs` (renamed to avoid clippy
      "module has same name" lint)
- [x] 2.3 Create `src/client/mod.rs` — `pub use mcp_client::McpClient;
      pub use handler::McpClientHandler;`
- [x] 2.4 `cargo test -p cast-mcp-client` — must pass

### Phase 3 — Create `generate/`

- [x] 3.1 Create `src/generate/params.rs` — move `ParamSpec`, `parse_params`,
      `camel_to_kebab` + `test_camel_to_kebab` unit test from `lib.rs`
- [x] 3.2 Create `src/generate/script.rs` — move `unix_now` (private),
      `generate_script` + its three unit tests from `lib.rs`
- [x] 3.3 Create `src/generate/mod.rs` — `pub use script::generate_script;
      pub(crate) use params::camel_to_kebab;`
- [x] 3.4 `cargo test -p cast-mcp-client` — must pass

### Phase 4 — Create `server_map.rs`

- [x] 4.1 Create `src/server_map.rs` — move `build_server_map` + its four
      unit tests from `lib.rs`
- [x] 4.2 `cargo test -p cast-mcp-client` — must pass

### Phase 5 — Create `commands/`

- [x] 5.1 Create `src/commands/list.rs` — move `list_tools_cmd` from `lib.rs`
- [x] 5.2 Create `src/commands/describe.rs` — move `describe_tool_cmd`
- [x] 5.3 Create `src/commands/status.rs` — move `status_cmd`
- [x] 5.4 Create `src/commands/call.rs` — move `call_tool_cmd` + `read_params`
- [x] 5.5 Create `src/commands/generate.rs` — move `generate_scripts_cmd`
- [x] 5.6 Create `src/commands/cli.rs` — move all Clap structs (`Cli`,
      `Commands`) + `run()` dispatch function from `main.rs`; also moved
      `print_json_error`
- [x] 5.7 Create `src/commands/mod.rs`
- [x] 5.8 `cargo test -p cast-mcp-client` — must pass

### Phase 6 — Slim `lib.rs` and `main.rs`

- [x] 6.1 Rewrite `lib.rs` to pure `pub mod` + re-exports (10 lines)
- [x] 6.2 Rewrite `main.rs` to 13 lines — just `Cli::parse()` + `run()`
- [x] 6.3 `cargo test -p cast-mcp-client` — must pass

### Phase 7 — Final verification

- [x] 7.1 `cargo clippy -p cast-mcp-client -- -D warnings` — clean
- [x] 7.2 `cargo test -p cast-mcp-client` — 44 tests pass (18 unit + 26 integration)
- [x] 7.3 Largest file is `generate/script.rs` at 310 lines (tests included);
      no production-code-only file exceeds 155 lines
- [x] 7.4 `lib.rs` = 10 lines, `main.rs` = 13 lines
