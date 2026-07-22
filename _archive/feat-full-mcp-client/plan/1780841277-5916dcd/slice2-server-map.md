---
status: complete
---

# Plan: Slice 2 — Server Map Logic

## Foreword
This executive plan covers the implementation and testing of **Slice 2: Server map logic** in `cast-mcp-client` and updating the runner logic in the `cast` crate. 
The goal is to properly resolve the `"cast"` server URL (incorporating CLI flag, environment variable, and config sources) and build a consolidated map of active remote servers, ensuring we transition cleanly from the old `CAST_MCP_URL` environment variable to the non-conflicting `CAST_MCP_CLIENT_URL` prefix across both crates.

All work follows strict TDD (Red-Green-Refactor) principles: writing a failing behavior-focused test first, then implementing the minimal code to satisfy it, and committing at each green step.

---

## Steps

### Part 1: Cast URL Resolution Logic (TDD Cycles)
- [x] **Step 1.1 (RED)**: Write unit tests in `crates/cast-mcp-client/src/lib.rs` for `resolve_cast_mcp_url` behavior:
  - CLI flag overrides environment variable
  - Environment variable `CAST_MCP_CLIENT_URL` overrides config entry `mcp.cast.url`
  - Returns `None` when no source provides a URL
- [x] **Step 1.2 (GREEN)**: Implement `pub fn resolve_cast_mcp_url(...)` in `crates/cast-mcp-client/src/lib.rs` to make the tests pass.
- [x] **Step 1.3 (COMMIT)**: Verify and commit.

### Part 2: Server Map Builder Logic (TDD Cycles)
- [x] **Step 2.1 (RED)**: Write unit tests in `crates/cast-mcp-client/src/lib.rs` for `build_server_map` behavior:
  - All `enabled: true` servers from config are included
  - Servers with `enabled: false` are excluded
  - If `cast_url` came from flag/env (not from config), it injects a bare-URL `"cast"` entry into the server map (overriding/removing headers defined in config for `"cast"`)
  - If `cast_url` came from the config itself, the full `"cast"` entry (including its headers) is preserved as-is
- [x] **Step 2.2 (GREEN)**: Implement `pub fn build_server_map(...)` in `crates/cast-mcp-client/src/lib.rs` to make the tests pass.
- [x] **Step 2.3 (COMMIT)**: Verify and commit.

### Part 3: Environment Variable Transition in `cast` Crate & `opencode.json`
- [x] **Step 3.1 (RED)**: Notice the failures or update the assertions in `crates/cast/src/dev/run.rs` tests to assert `CAST_MCP_CLIENT_URL` instead of `CAST_MCP_URL`.
- [x] **Step 3.2 (GREEN)**: Update `crates/cast/src/dev/run.rs` to inject `CAST_MCP_CLIENT_URL` instead of `CAST_MCP_URL` so all tests pass.
- [x] **Step 3.3 (GREEN)**: Update `opencode.json` to map `CAST_MCP_CLIENT_URL`.
- [x] **Step 3.4 (COMMIT)**: Verify and commit.
