# Execution Roadmap: MCP Client - Minimalist JSON

## Slice 1 & 2: Discovery (DONE)
- [x] URL Resolution and `cast mcp list`.

## Slice 3: Inspection (`describe`) (DONE)
- [x] Add `Describe` variant to `McpCommands` in `src/commands/cli.rs`.
- [x] Implement `cast mcp describe <tool>` in `src/commands/mcp/mod.rs`.
- [x] Simple pretty-printer for the `inputSchema`.
- [x] **RED**: Test `describe` output with a mock server.

## Slice 4: Execution (`call`)
- [x] Add `Call` variant to `McpCommands` in `src/commands/cli.rs`.
- [x] Implement logic to read JSON from argument or stdin.
- [x] Implement `mcp_client.call_tool` to send the payload.
- [x] **RED**: Integration test for a successful tool call.
- [x] **RED**: Integration test for a failed tool call (verifying we show the server error).
- [x] **RED**: Integration test for piped stdin input.
- [x] **GREEN**: Final wiring and clean exit.
