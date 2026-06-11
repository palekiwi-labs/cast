# MCP Server Overview

`cast` includes a built-in Model Context Protocol (MCP) server that provides tools and documentation to the coding agent.

## Features

- **Tool Execution**: Allows the agent to run approved tools on your host (e.g., searching files, running tests).
- **Embedded Documentation**: Serves the contents of the `docs/` directory to the agent via the MCP protocol.
- **Resource Management**: Provides a structured way for agents to discover project context.

## How it works

When an agent starts, `cast` launches an Axum-based HTTP server. The agent connects to this server and can:
1. **List Tools**: See what tools are available (defined in `cast.json`).
2. **Call Tools**: Execute a tool with specific arguments.

## Next Steps

- [Tool Configuration](configuration.md): Learn how to define tools in `cast.json`.
- [Client Usage](client.md): How to interact with the server from the command line.
