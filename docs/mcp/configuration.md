# MCP Configuration in `cast`

The `cast` MCP server allows you to define dynamic tools that can run commands on your host or in your development environment.

## Configuration

MCP tools are defined in the `mcp.tools` section of your `cast.json` file.

### Example Configuration

```json
{
  "mcp": {
    "port": 3000,
    "tools": {
      "run_tests": {
        "description": "Run the test suite",
        "host_cmd": ["cargo", "test"],
        "parameters": {
          "type": "object",
          "properties": {
            "test_name": {
              "type": "string",
              "description": "Name of the test to run"
            }
          }
        }
      }
    }
  }
}
```

### Schema Fields

- `description`: A human-readable description of what the tool does.
- `host_cmd`: The command to execute on the host. This is an array of strings.
- `parameters`: A JSON Schema (Draft 7) defining the arguments the tool accepts.

## Usage

Start the MCP server using the CLI:

```bash
cast mcp start
```

Once started, any MCP-compatible client (like Claude Desktop) can connect to the server and use the defined tools.
