# Project Log

## [e230c3a] Research complete: cast-mcp-client API and output patterns

- **Found:** CLI commands: list, describe, call
- **Found:** Current output is predominantly unstructured text via println!
- **Found:** Underlying data models in rmcp (Tool, CallToolResult, Content) already support Serde serialization.
- **Found:** Proposed structured output can directly leverage these models.

## [edcfd67] Implemented JSON output for cast-mcp-client

All 6 TDD slices implemented and committed. Two commits on feat/mcp-client-json.

- **Found:** rmcp models already derive Serialize so changes were minimal
- **Found:** print_tool_schema deleted — replaced entirely by serde
- **Found:** main.rs required no logic changes, only cargo fmt reformatting
- **Decided:** JSON by default, no flags
- **Decided:** Raw MCP models (no envelope)
- **Decided:** Errors as JSON on stderr with non-zero exit
- **Decided:** Tool-level isError reflected in JSON, no process crash

## [ff9c8e7] Fixed double-error reporting and unstructured stderr leakage

- **Found:** anyhow's default handler was printing a second 'Error: ...' line after our JSON
- **Found:** all non-tool-not-found errors bypassed JSON formatting entirely
- **Decided:** main() owns the error output contract — single catch-all formats all errors as JSON
- **Decided:** lib.rs command functions return plain anyhow errors with no stderr side effects
- **Decided:** clap parse errors intentionally left as plain text — caller contract violation, not a runtime error

