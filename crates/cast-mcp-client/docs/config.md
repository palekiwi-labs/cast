# Client Configuration

The client looks for a `cast-mcp-client.json` file.

## File Locations

1. **Global**: `~/.config/cast/cast-mcp-client.json`
2. **Project**: `./cast-mcp-client.json`

## Schema

```json
{
  "mcp": {
    "my_server": {
      "url": "http://localhost:8080/mcp",
      "headers": {
        "Authorization": "Bearer {env:API_KEY}"
      },
      "enabled": true
    }
  }
}
```

## Environment Substitution

Values in the `headers` map support `{env:VAR_NAME}` syntax, which the client
replaces with the corresponding environment variable at runtime.

## The `cast` Server

The client always includes a default server named `"cast"`. Its URL can be
overridden by the `CAST_MCP_URL` environment variable or the `--cast-mcp-url`
CLI flag.
