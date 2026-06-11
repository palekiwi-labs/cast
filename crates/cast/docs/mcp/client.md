# Using the MCP Client

When you are inside a `cast` agent sandbox, you can use `cast-mcp-client` to
interact with the built-in MCP server.

## Common Tasks

### List Available Tools

To see what tools are configured for the current project:

```bash
cast-mcp-client list
```

### Describe a Tool

To see the input schema for a specific tool on the `cast` server:

```bash
cast-mcp-client describe cast list_cast_documentation
```

### Call a Tool

To execute a tool with JSON arguments:

```bash
cast-mcp-client call cast fetch_cast_documentation '{"id": "mcp/configuration"}'
```

## How it Connects

The client uses the `CAST_MCP_URL` environment variable, which `cast` sets
automatically inside the sandbox. By default, it points to the host's bridge IP
and the deterministic port assigned to the session.

For more details on the client, see the [cast-mcp-client documentation].

[cast-mcp-client documentation]: ../../cast-mcp-client/docs/README.md
