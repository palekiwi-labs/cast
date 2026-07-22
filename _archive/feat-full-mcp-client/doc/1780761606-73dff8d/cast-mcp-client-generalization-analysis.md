# Research Report: Analysis of `cast-mcp-client` for General MCP Server Support

## Research Question
Analyze the current `cast-mcp-client` implementation and determine how to support a general client configuration (`cast-mcp-client.json`) for multiple MCP servers (remote and stdio).

## Findings

### 1. Current Implementation Analysis
The current `cast-mcp-client` is a specialized HTTP/SSE client for Model Context Protocol (MCP) servers.

- **Transport**: Hardcoded to use `StreamableHttpClientTransport` from the `rmcp` crate.
- **Connection**: Only supports HTTP connections. The URL is resolved with the following priority:
  1. `--url` flag
  2. `CAST_MCP_URL` environment variable
  3. Default: `http://127.0.0.1:8080/mcp`
- **CLI**: Supports `list`, `describe`, and `call` commands, but they are all tied to a single server instance per execution.
- **Dependencies**: Uses `rmcp` version 1.6.0, which provides the core MCP protocol implementation and transport layers.

### 2. `rmcp` Library Capabilities
An analysis of the `rmcp` library reveals it already supports the necessary transports and authentication mechanisms:

- **Stdio Transport**: Supported via `rmcp::transport::child_process::TokioChildProcess`. It can spawn a local command and communicate over stdin/stdout.
- **HTTP/SSE Transport**: Supported via `StreamableHttpClientTransport`, including support for:
  - **Custom Headers**: Can be provided via `StreamableHttpClientTransportConfig`.
  - **OAuth 2.0**: Full support for Authorization Code and Client Credentials flows, including token storage and automatic refresh.
- **Transport Agnostic**: The library uses an `IntoTransport` trait, allowing it to work with any `(AsyncRead, AsyncWrite)` pair, including Unix sockets and TCP.

### 3. Proposed Configuration Schema (`cast-mcp-client.json`)
Following the pattern of `opencode.json`, the configuration should allow defining multiple servers with their specific settings.

```jsonc
{
  "mcp": {
    "everything": {
      "type": "local",
      "command": ["npx", "-y", "@modelcontextprotocol/server-everything"],
      "environment": {
        "DEBUG": "true"
      }
    },
    "jira": {
      "type": "remote",
      "url": "https://jira.example.com/mcp",
      "headers": {
        "Authorization": "Bearer {env:JIRA_TOKEN}"
      },
      "enabled": true
    }
  }
}
```

#### Key Configuration Fields (inspired by OpenCode):
- **`type`**: `"local"` or `"remote"`.
- **`command`**: Array of strings for local servers.
- **`environment`**: Key-value pairs for local server environment variables.
- **`url`**: The endpoint for remote servers.
- **`headers`**: Custom HTTP headers for remote servers (supporting `{env:VAR}` substitution).
- **`oauth`**: OAuth configuration for remote servers.
- **`enabled`**: Boolean to toggle servers.
- **`timeout`**: Connection and request timeout in milliseconds.
