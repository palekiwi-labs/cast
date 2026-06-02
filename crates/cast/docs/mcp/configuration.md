# MCP Configuration in `cast`

The `cast` MCP server allows you to define dynamic tools that can run commands on your host or in your development environment. These tools are defined in the `mcp.tools` section of your `cast.json` file or optionally in a separate `cast-mcp.json` file.

When both files are present, `cast-mcp.json` takes precedence and its contents are merged with `cast.json`. This allows you to keep your main configuration clean by moving MCP-specific settings to their own file.

## Tool Definition

A tool definition consists of its metadata, the command to execute, and how to map MCP parameters to command-line arguments.

### Schema Fields

- `port`: The port the MCP server should listen on.
- `hostname` (optional): The hostname to bind to (defaults to `127.0.0.1`).
- `tools`: A map of tool names to their definitions.

#### Tool Schema Fields

- `description`: A human-readable description of what the tool does.
- `command`: The base command to execute (e.g., `cargo`, `npm`, `docker`).
- `args`: An array of `ArgTemplate` objects defining the arguments passed to the command.
- `parameters`: A JSON Schema (Draft 7) defining the arguments the tool accepts from the agent.
- `working_dir` (optional): The directory where the command should be executed.
- `env` (optional): Environment variable configuration.
  - `inherit`: List of environment variables to inherit from the host. Note that `PATH` and `TMPDIR` are **always inherited** for system compatibility.
  - `set`: Map of environment variables to set specifically for this tool.

### Example Configuration

```json
{
  "mcp": {
    "port": 3000,
    "hostname": "127.0.0.1",
    "tools": {
      "run_tests": {
        "description": "Run the test suite",
        "command": "cargo",
        "args": [
          "test",
          { "if_present": "test_name", "args": ["--test", "{test_name}"] }
        ],
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

## Argument Templates

Arguments can be simple strings (literals) or conditional blocks.

### Literals and Placeholders

Literal strings in the `args` array can contain placeholders:

- `{name}`: Replaced by the value of the parameter `name`. If the parameter is missing, the placeholder text is removed. If the entire argument string was just the placeholder (e.g. `"{name}"`), the argument is **omitted** entirely.
- `{...name}`: The **spread operator**. If the parameter `name` is an array, it expands into multiple CLI arguments (one per array element).

### Conditional Blocks

Conditional blocks allow you to include a set of arguments only if certain conditions are met. If both `if_present` and `if_true` are provided, both conditions must be met (logical AND).

- `if_present`: Include `args` if the specified parameter key exists and is not null.
- `if_true`: Include `args` if the specified parameter key evaluates to a boolean `true`.

Example of a conditional block:

```json
{
  "if_present": "verbose",
  "args": ["--verbose", "--log-level", "debug"]
}
```

## Built-in Tools

`cast` also provides built-in tools for exploring its own documentation:

- `list_cast_documentation`: Lists available documentation files.
- `fetch_cast_documentation`: Retrieves the content of a specific documentation file.

## Usage

Start the MCP server using the CLI:

```bash
cast mcp start
```

Once started, any MCP-compatible client (like Claude Desktop) can connect to the server and use the defined tools. The server will automatically validate all incoming tool calls against the JSON Schemas defined in your `parameters` fields.

### Execution Environment

For security and reproducibility:
- `stdin` is set to `null` (tools cannot read interactive input).
- The environment is cleared of all host variables except those explicitly inherited or set.
- `PATH` and `TMPDIR` are always preserved to ensure basic system commands work.
