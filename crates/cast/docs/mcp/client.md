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
inside the sandbox when the project configures MCP. It points to the host's
bridge IP and the configured `mcp.port`.

The variable is only set when some configuration source declares an `mcp`
block (see [Configuration Reference][config-reference]). In a project with
no block, `CAST_MCP_URL` is absent, and `cast-mcp-client` omits the `cast`
server entirely rather than reporting an unreachable one — so `status` and
`list` read as "not configured" instead of as a failure.

Other consumers that template the variable need to account for the absence.
An opencode remote entry such as:

```json
{ "mcp": { "mcp-cast": { "type": "remote", "url": "{env:CAST_MCP_URL}" } } }
```

resolves to an empty URL in a project with no `mcp` block. Disable that
entry in MCP-less projects, or keep it in the project-local opencode config
alongside the `mcp` block that justifies it.

For more details on the client, see the [cast-mcp-client documentation].

[cast-mcp-client documentation]: ../../cast-mcp-client/docs/README.md
[config-reference]: ../config/reference.md
