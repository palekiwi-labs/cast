---
status: complete
---

# Proposal for a new client feature: generate scripts from tool schema

## Context

`cast-mcp-client` is aware of the tools it offers and the precise schema
of every tool. Calling these tools manually on the CLI by agents is 
difficult though - it is verbose (`cast-mcp-client call my-server my-tool '{...}`),
the experience is very clunky and requires knowledge and documentation.

## Proposed solution

Support a command that generates script wrappers for every available tool.

The MCP protocol defines a standard schema for tools. We know such things as:
- name
- description
- params
- a standard output format

Can we automate the generation of bash script wrappers that would allow AI agents
direct ergonomic and easy to discover access to the expose tools as command line
utilities?

### Example API:

```bash
cast-mcp-client generate [servers] [tools] --dir <output-dir>
```

### Example script structure

```bash
# script description generated from the schema (name, description, etc)

# argument parsing - generated from the schema
# must support `--help` flag with auto-generated help content

# output parsing
# MCP generates output that is made for consumption via MCP adapters
# since we are wrapping it as a CLI script, we need to parse the output,
# handle errors and properly print output to stdout/stderr
#
# {
#   "content": [
#     {
#       "type": "text",
#       "text": "Available cast documentation:\n\n- `mcp/configuration`\n\nUse `fetch_cast_documentation` tool to read an entry."
#     }
#   ],
#   "isError": false
# }
```
