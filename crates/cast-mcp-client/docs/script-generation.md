# Script Generation

`cast-mcp-client` can generate Bash wrappers for your MCP tools, making them feel like native CLI commands.

## How to Generate

Run the `generate` command and pipe the output to a directory:
```bash
mkdir -p .tools
cast-mcp-client generate > .tools/gen.sh
# Note: the generate command currently prints the scripts to stdout or a directory depending on implementation.
```

## Anatomy of a Wrapper

A generated wrapper handles:
1. **Argument Parsing**: Converts CLI flags to the JSON structure required by the MCP tool.
2. **Validation**: Ensures required arguments are present.
3. **Invocation**: Calls `cast-mcp-client call` under the hood.
4. **Output Handling**: Uses `jq` to process the results.

## Prerequisites

Generated scripts require `bash` and `jq` to be available in the environment.
