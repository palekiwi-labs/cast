# Implementation Plan: MCP Client

This plan covers the construction of a stateless MCP client for `cast`.

## Architectural Approach

We use the `rmcp` crate's `StreamableHttpClientTransport` to handle the SSE-based protocol. The client is split into two logical phases: Discovery (`list`) and Execution (`call`).

### Phase 1: Infrastructure & Discovery (`feat/mcp-list`)

1. **URL Resolution (`src/commands/mcp/client.rs`)**
   - Implement logic to resolve the MCP server URL.
   - Priority: `--url` flag > `CAST_MCP_URL` env > `cast.json` (if on host) > `http://host.docker.internal:8080/mcp` (if in container).

2. **Transport & Client Setup**
   - Initialize `StreamableHttpClientTransport` using `reqwest`.
   - Implement a basic `McpClient` wrapper that handles the `initialize` handshake.

3. **Subcommand: `cast mcp list`**
   - Call `tools/list`.
   - Format and print tool names and descriptions to the terminal.

4. **Inspection: `cast mcp describe <name>`**
   - Fetches `tools/list` and filters for the requested tool.
   - Pretty-prints the `inputSchema` (JSON) to help users craft their call payloads.

### Phase 2: Execution (`feat/mcp-call`)

1. **Pure JSON Input**
   - Instead of dynamic flag parsing, the client accepts a single JSON argument.
   - Supports:
     - Inline JSON: `cast mcp call tool '{"key": "val"}'`
     - Stdin (piping): `echo '{}' | cast mcp call tool`
   - No client-side type coercion or schema validation (server handles this).

2. **Subcommand: `cast mcp call <name> [JSON]`**
   - Perform full lifecycle: `initialize` -> `call_tool`.
   - Result handling: Print text content and handle tool-reported errors.

3. **Error Reporting**
   - Map JSON-RPC errors (e.g., `-32602` Invalid Params) to clean terminal diagnostics.
