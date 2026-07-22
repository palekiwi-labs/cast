# Command: mcp client (list & call)

---

## Context

Implement an integrated MCP (Model Context Protocol) client into the `cast` CLI. This allows users (and scripts like git hooks) inside containers to discover and execute tools exposed by the `cast` MCP server running on the host.

## Design Strategy: Stateless Smart-Relay

The client acts as a lightweight transport between the CLI user and the MCP server.

### Key Features
- **Zero-Config Discovery**: Automatically detects the server endpoint (defaulting to `host.docker.internal` inside containers).
- **Dynamic Introspection**: Fetches tool definitions and schemas on-demand for `list` and `--help`.
- **Stateless Operation**: No local caching of schemas; always uses the server as the single source of truth.
- **Ergonomic Call Syntax**: Maps CLI flags and key-value pairs to JSON parameters, with type coercion based on the server-provided schema.

## Execution Branches
- `feat/mcp-list`: Infrastructure, URL resolution, and tool listing.
- `feat/mcp-call`: Argument mapping, tool execution, and error reporting.

## References
- Research: `.mem/feat-mcp-client/doc/mcp-client-http-streaming.md`
- MCP Server Implementation: `.mem/feat-mcp/spec/index.md`
