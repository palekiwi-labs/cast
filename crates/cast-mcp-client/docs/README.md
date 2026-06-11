# cast-mcp-client Documentation

`cast-mcp-client` is a lightweight Model Context Protocol (MCP) client. While
primarily used inside `cast` sandboxes, it can be used to interact with any MCP
server.

## Sections

- **[Usage](usage.md)**: Guide to subcommands and examples.
- **[Configuration](config.md)**: How to configure remote MCP servers.
- **[Script Generation](script-generation.md)**: Automatically generating Bash
  wrappers for MCP tools.

## Relationship to `cast`

`cast` starts a built-in MCP server for every agent session. `cast-mcp-client`
is the tool used by the agent (or you) to query and invoke tools on that
server.
