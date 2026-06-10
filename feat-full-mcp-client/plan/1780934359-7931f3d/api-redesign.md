---
status: done
---

## Status: Complete

## Foreword

This plan implements the API redesign captured in the todo artifact
`.mem/feat-full-mcp-client/todo/1780934359-7931f3d/api-redesign.md`.

It covers three interconnected changes to `cast-mcp-client`:

1. `list` output format: flat prefixed array → nested object keyed by server name
2. `list` server filter: `--server <name>` flag → positional varargs `[servers...]`
3. `describe` / `call` tool reference: single `server/tool` string → two positional args `<server> <tool>`

All three changes are coupled (lib signatures, main wiring, tests all move together),
so they are implemented as a single TDD cycle with one atomic commit.

Base branch context: `feat/full-mcp-client` at commit `3921a90`.
Test count before this work: 39 (18 unit + 21 integration).

---

## Steps

### RED — Update tests first

- [x] In `tests/mcp_client_test.rs`:
  - [x] `test_mcp_list_subcommand_output`: assert `json` is an object; assert `json["cast"][0]["name"] == "dummy_tool"`
  - [x] `test_mcp_describe_subcommand_output`: change args to `["describe", "cast", "dummy_tool", "--cast-mcp-url", &url]`
  - [x] `test_mcp_describe_unknown_tool_fails`: change args to `["describe", "cast", "nonexistent_tool", "--cast-mcp-url", &url]`
  - [x] `test_mcp_call_inline_json`: change args to `["call", "cast", "dummy_tool", r#"{"message":"hello"}"#, "--cast-mcp-url", &url]`
  - [x] `test_mcp_call_stdin_json`: change args to `["call", "cast", "dummy_tool", "-", "--cast-mcp-url", &url]`
  - [x] `test_mcp_call_unknown_tool_fails`: change args to `["call", "cast", "nonexistent_tool", "{}", "--cast-mcp-url", &url]`
  - [x] `test_mcp_call_tool_error_in_json`: change args to `["call", "cast", "error_tool", "{}", "--cast-mcp-url", &url]`
  - [x] `test_list_prefixed_tools_single_server` → rename to `test_list_nested_single_server`: assert `json` is object; assert `json["cast"][0]["name"] == "dummy_tool"`
  - [x] `test_list_filter_by_server`: change args to `["list", "sentry"]`; assert `json["sentry"][0]["name"] == "dummy_tool"`
  - [x] `test_list_unknown_server_fails`: change args to `["list", "ghost"]`
  - [x] `test_describe_server_slash_tool_format` → rename to `test_describe_two_positional_args`: change args to `["describe", "cast", "dummy_tool"]`
  - [x] `test_call_server_slash_tool_format` → rename to `test_call_two_positional_args`: change args to `["call", "cast", "dummy_tool", r#"{"message":"hello"}"#]`
  - [x] `test_routing_no_separator_fails`: **delete** (clap handles missing required arg natively; no custom JSON error needed)
  - [x] `test_routing_unknown_server_fails`: change args to `["describe", "ghost", "dummy_tool"]`
  - [x] `test_list_ignores_unreachable_server`: assert `json["good"][0]["name"] == "dummy_tool"`
  - [x] `test_list_reads_project_config`: assert `json["cast"][0]["name"] == "dummy_tool"`
  - [x] `test_list_empty_config_returns_empty_array` → rename to `test_list_empty_config_returns_empty_object`: assert stdout trims to `"{}"`

### GREEN — Update lib.rs

- [x] Delete `parse_server_tool` (no longer needed)
- [x] Update `list_tools_cmd` signature: `servers: Vec<String>` instead of `server_filter: Option<String>`
  - Filter targets: if `servers` is non-empty, validate each name against the map and build target subset; if empty, use all entries
  - Remove name-prefixing (`t.name = format!("{}/{}", ...)`)
  - Build `HashMap<String, Vec<Tool>>` output instead of `Vec<Tool>`
  - Serialize as nested object: `{"server_a": [...], "server_b": [...]}`
  - Empty map case: print `{}` instead of `[]`
- [x] Update `describe_tool_cmd` signature: `(server_name: String, tool_name: String, server_map: ...)` — remove `parse_server_tool` call, use args directly
- [x] Update `call_tool_cmd` signature: `(server_name: String, tool_name: String, params: Option<String>, server_map: ...)` — same

### GREEN — Update main.rs

- [x] `List` variant: replace `server: Option<String>` with `servers: Vec<String>` as a positional vararg (`#[arg()]` with no `long`, allow multiple)
- [x] `Describe` variant: replace single `tool_name: String` with `server_name: String` + `tool_name: String` (two positional args)
- [x] `Call` variant: replace single `tool_name: String` with `server_name: String` + `tool_name: String`; `params: Option<String>` remains as 3rd positional
- [x] Update all match arms to pass `server_name` and `tool_name` separately to the lib functions
- [x] `List` match arm: pass `servers` (Vec) instead of `server` (Option)

### Verify

- [x] `cargo test -p cast-mcp-client` — all tests pass (expect ~38: one test deleted)
- [x] Quick smoke-test: `cargo run -p cast-mcp-client -- list --help` to confirm clap output

### Commit

- [x] Single atomic commit: `refactor(mcp-client): redesign CLI API — nested list output, two-arg describe/call`
