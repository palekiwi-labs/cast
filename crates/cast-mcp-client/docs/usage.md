# Usage Guide

`cast-mcp-client` provides several subcommands for interacting with MCP servers.

## Commands

### `list`
Lists all tools exposed by the configured MCP servers.
```bash
cast-mcp-client list
```

### `describe <tool>`
Shows the JSON Schema input for a specific tool.
```bash
cast-mcp-client describe search_files
```

### `call <tool> <args>`
Invokes a tool with JSON arguments.
```bash
cast-mcp-client call search_files '{"query": "Agent"}'
```

### `status`
Checks the health of all configured MCP servers.

### `generate`
Generates Bash script wrappers for every tool on the configured servers.

## Global Flags

- `--cast-mcp-url`: Override the URL for the default `"cast"` server.
- `--env`: Show errors in a more verbose format.
