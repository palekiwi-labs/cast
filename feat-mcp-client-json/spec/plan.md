# Plan: cast-mcp-client JSON Output

## Goal

Replace all unstructured `println!` output in `cast-mcp-client` with structured JSON
as the default and only output format. No flags or modes — JSON always.

## Design Decisions

- **JSON by default**: No `--json` flag. All commands always emit JSON to `stdout`.
- **No human-readable fallback**: The tool is primarily for programmatic/agent use.
- **Raw MCP models**: No custom envelope wrapper. Output the standard MCP types directly.
- **Errors on `stderr`**: Structured JSON errors go to `stderr`. Non-zero exit codes on failure.
- **Diagnostics always on `stderr`**: Logs, tracing, and progress messages never pollute `stdout`.

## Output Schemas

### `list` → `Array<Tool>`
```json
[
  {
    "name": "read_file",
    "description": "Read content from a file",
    "inputSchema": {
      "type": "object",
      "properties": { "path": { "type": "string" } },
      "required": ["path"]
    }
  }
]
```

### `describe` → `Tool`
```json
{
  "name": "read_file",
  "description": "Read content from a file",
  "inputSchema": { ... }
}
```

### `call` → `CallToolResult`
```json
{
  "content": [
    { "type": "text", "text": "File contents here..." }
  ],
  "isError": false
}
```

### Error (on `stderr`)
```json
{
  "error": {
    "code": "TOOL_NOT_FOUND",
    "message": "Unknown tool 'foo'. Run 'cast-mcp-client list' to see available tools."
  }
}
```

## Implementation Steps

### 1. `src/lib.rs` — Refactor command functions

- **`list_tools_cmd`**: Replace `println!` loop with `serde_json::to_string_pretty(&tools)`.
- **`describe_tool_cmd`**: Replace `print_tool_schema(...)` call with `serde_json::to_string_pretty(&tool)`.
- **`call_tool_cmd`**: Replace content-iterating `match` block with `serde_json::to_string_pretty(&result)`.
- **Remove `print_tool_schema`**: This function becomes dead code and should be deleted.
- **Error helper**: Add a `print_json_error(code, message)` fn that writes structured JSON to `stderr`.
  Use this in `describe_tool_cmd` for the "unknown tool" case.

### 2. `src/main.rs` — No changes needed

The `main.rs` entrypoint requires no structural changes. The JSON output change is
entirely encapsulated in `lib.rs`.

### 3. `tests/mcp_client_test.rs` — Update & extend tests

Existing tests assert on human-readable text output and must be updated:

- **`test_mcp_list_subcommand_output`**: Assert valid JSON array; check `.[0].name == "dummy_tool"`.
- **`test_mcp_describe_subcommand_output`**: Assert valid JSON object; check `.name`, `.inputSchema`.
- **`test_mcp_describe_unknown_tool_fails`**: Assert non-zero exit + JSON error object on `stderr`.
- **`test_mcp_call_inline_json`**: Assert valid `CallToolResult` JSON; check `.content[0].text`.
- **`test_mcp_call_stdin_json`**: Same as above, different input path.

New tests to add:
- **`test_mcp_list_json_is_valid`**: Parse `stdout` as `Vec<Tool>` and assert fields.
- **`test_mcp_call_returns_call_tool_result`**: Parse `stdout` as `CallToolResult` and assert.
- **`test_mcp_stdout_is_clean_on_success`**: Assert no non-JSON bytes on `stdout` (no log leakage).

## Files Changed

| File | Change |
| :--- | :--- |
| `crates/cast-mcp-client/src/lib.rs` | Replace print logic with JSON serialization; add error helper; remove `print_tool_schema` |
| `crates/cast-mcp-client/tests/mcp_client_test.rs` | Update existing tests; add new JSON-specific tests |
| `crates/cast-mcp-client/src/main.rs` | No changes required |
