# Intent: MCP Documentation Serving

Implement a feature in the `cast` MCP server to serve project documentation to AI agents using the MCP "Resources" pattern.

## Scope
- Serve Markdown documentation files stored in the repository via the MCP server.
- Use the `cast://docs/` URI scheme for resource identification.
- Ensure documentation is embedded in the binary for portable distribution.
- Initial documentation focus: MCP configuration and dynamic tool schemas.

## Requirements
- Support `list_resources` to allow agents to discover available documentation.
- Support `read_resource` to allow agents to fetch the content of specific documentation.
- Use `include_str!` for compile-time embedding of documentation files.
- Metadata (name, description, mime type) must be provided for each resource.

## Prerequisites
- Existing MCP server implementation in `src/commands/mcp/`.
- `rmcp` crate version 1.6.0.
