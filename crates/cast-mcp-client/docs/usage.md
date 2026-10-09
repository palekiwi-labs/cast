# Usage Guide

`cast-mcp-client` provides several subcommands for interacting with MCP
servers.

## Commands

### `list`

Lists all tools exposed by the configured MCP servers.

```bash
cast-mcp-client list
```

### `describe <server> <tool>`

Shows the JSON Schema input for a specific tool on a specific server.

```bash
cast-mcp-client describe cast list_cast_documentation
```

### `call <server> <tool> <args>`

Invokes a tool with JSON arguments.

```bash
cast-mcp-client call cast fetch_cast_documentation '{"id": "mcp/configuration"}'
```

### `status`

Checks the health of all configured MCP servers.

### `generate`

Generates Bash script wrappers for every tool on the configured servers.

## Subcommand Flags

Every subcommand accepts the following option:

- `--cast-mcp-url <URL>`: Override the URL for the default `"cast"` server.
  Takes precedence over the `CAST_MCP_URL` environment variable and config.

The flag belongs to each subcommand, not the root command, so it must come
after the subcommand name:

```bash
cast-mcp-client list --cast-mcp-url http://localhost:8080/mcp
```
