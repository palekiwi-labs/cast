---
status: complete
---

## Foreword

This executive plan covers **Slice 6 (S6): describe/call — server/tool format + routing**.

Master plan reference: `plan/index.md` → S6 section.

**Goal:** Update `describe_tool_cmd` and `call_tool_cmd` in `src/lib.rs` to:
1. Require tool arguments in `server/tool` format (e.g. `cast/dummy_tool`).
2. Parse the prefix to identify the target server.
3. Look up that server in the server map and route the request to it using the bare tool name.
4. Fail fast with a structured error for bare tool names or unknown server names.

**Current state (post-S5):**
- Both `describe_tool_cmd` and `call_tool_cmd` accept a bare `tool_name: String` and use
  `pick_server()` to select a server (preferring "cast", else first entry).
- No `server/tool` parsing exists anywhere.
- Existing integration tests in `tests/mcp_client_test.rs` pass bare tool names
  (`"dummy_tool"`) and a `--cast-mcp-url` flag; these will break and must be updated.
- `pick_server()` in `lib.rs` becomes dead code once S6 is wired up; it can be removed or
  left (it currently has no tests directly, only indirect use).

**Prerequisites:** S1–S5 committed and passing (`cargo test`).

---

## Steps

### Red phase — write failing tests first

- [x] **Step 1 — Add `test_describe_server_slash_tool_format`**
  In `tests/mcp_client_test.rs`, after the S5 block, add a new `#[tokio::test]`:
  - Spawn `spawn_mock_server()` for a "cast" server.
  - Write a `cast-mcp-client.json` in a `tempdir` with `{ "mcp": { "cast": { "url": "<url>" } } }`.
  - Run: `cast-mcp-client describe cast/dummy_tool` with `current_dir(tmpdir)` and `env_remove("CAST_MCP_URL")`.
  - Assert: exit success, stdout is valid JSON with `json["name"] == "dummy_tool"` (bare name in output).

- [x] **Step 2 — Add `test_call_server_slash_tool_format`**
  In `tests/mcp_client_test.rs`, add a `#[tokio::test]`:
  - Spawn `spawn_mock_server()` for "cast".
  - Write a `cast-mcp-client.json` in a `tempdir` with "cast" server.
  - Run: `cast-mcp-client call cast/dummy_tool '{"message":"hello"}' ` with `current_dir(tmpdir)` and `env_remove("CAST_MCP_URL")`.
  - Assert: exit success, stdout JSON has `content[0].text` containing `"echo: hello"`.

- [x] **Step 3 — Add `test_routing_no_separator_fails`**
  In `tests/mcp_client_test.rs`:
  - Spawn `spawn_mock_server()`.
  - Write config in `tempdir`.
  - Run: `cast-mcp-client describe dummy_tool` (no slash prefix) with `current_dir(tmpdir)` and `env_remove("CAST_MCP_URL")`.
  - Assert: exit failure, stderr JSON has `error.code == "COMMAND_ERROR"` and message contains `"server/tool"`.

- [x] **Step 4 — Add `test_routing_unknown_server_fails`**
  In `tests/mcp_client_test.rs`:
  - Spawn `spawn_mock_server()`.
  - Write config with only "cast" server in `tempdir`.
  - Run: `cast-mcp-client describe ghost/dummy_tool` with `current_dir(tmpdir)` and `env_remove("CAST_MCP_URL")`.
  - Assert: exit failure, stderr JSON has `error.code == "COMMAND_ERROR"` and message contains `"ghost"`.

- [x] **Step 5 — Verify RED** — run `cargo test -p cast-mcp-client` and confirm the four new tests fail; all existing tests still pass.

---

### Green phase — implement routing

- [x] **Step 6 — Add `parse_server_tool()` helper in `src/lib.rs`**
  ```rust
  /// Split a `"server/tool"` reference into `(server_name, tool_name)`.
  /// Returns an error if there is no `/` separator.
  fn parse_server_tool(reference: &str) -> anyhow::Result<(&str, &str)> {
      reference
          .split_once('/')
          .ok_or_else(|| anyhow::anyhow!(
              "Tool reference must be in 'server/tool' format (got '{}').",
              reference
          ))
  }
  ```

- [x] **Step 7 — Rewrite `describe_tool_cmd` in `src/lib.rs`**
  Replace the current `pick_server` + bare-name lookup with:
  1. Call `parse_server_tool(&tool_name)` → `(server_name, bare_tool_name)`.
  2. Lookup `server_map.get(server_name)` → error if absent (`"Unknown server '{}'. …"`).
  3. Connect to that server only.
  4. List tools and find `bare_tool_name` (not the prefixed name).
  5. Print the tool JSON (the `name` field remains bare/un-prefixed — this is `describe`, not `list`).

- [x] **Step 8 — Rewrite `call_tool_cmd` in `src/lib.rs`**
  Replace the current `pick_server` with:
  1. Call `parse_server_tool(&tool_name)` → `(server_name, bare_tool_name)`.
  2. Lookup `server_map.get(server_name)` → error if absent.
  3. Connect to that server and call `bare_tool_name` (not the slash-prefixed form).

- [x] **Step 9 — Remove `pick_server()` from `src/lib.rs`**
  The helper is now dead code. Delete it to keep the codebase clean.
  (If the compiler already warns about it as dead_code, this confirms it is unused.)

- [x] **Step 10 — Update existing `describe` and `call` integration tests in `tests/mcp_client_test.rs`**
  The following tests currently pass a bare tool name via `--cast-mcp-url`. They must be updated
  to use the `server/tool` format instead (the "cast" server is injected via `--cast-mcp-url`):
  - `test_mcp_describe_subcommand_output`: `"dummy_tool"` → `"cast/dummy_tool"`
  - `test_mcp_describe_unknown_tool_fails`: `"nonexistent_tool"` → `"cast/nonexistent_tool"`
  - `test_mcp_call_inline_json`: `"dummy_tool"` → `"cast/dummy_tool"`
  - `test_mcp_call_stdin_json`: `"dummy_tool"` → `"cast/dummy_tool"`
  - `test_mcp_call_unknown_tool_fails`: `"nonexistent_tool"` → `"cast/nonexistent_tool"`
  - `test_mcp_call_tool_error_in_json`: `"error_tool"` → `"cast/error_tool"`

  Note: the `--cast-mcp-url` flag path puts the server in the map under the key `"cast"`,
  so `"cast/dummy_tool"` will resolve correctly after the S6 implementation.

- [x] **Step 11 — Verify GREEN** — run `cargo test -p cast-mcp-client` and confirm all tests pass, including the four new ones.

---

### Commit

- [x] **Step 12 — Commit**
  ```
  feat(mcp-client): S6 — describe/call require server/tool format with routing
  ```
  Stage only `crates/cast-mcp-client/` files.

- [x] **Step 13 — Log milestone** with `mem-log`.
