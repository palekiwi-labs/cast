# Command: mcp

---

## Context

Implement a built-in MCP (Model Context Protocol) server that a user can start with a new subcommand `cast mcp start`. The purpose is to allow sandboxed agents inside containers to execute whitelisted, structured commands on the host.

## Design Strategy: Semantic Tools

Instead of a generic shell interceptor, we implement **Semantic MCP Tools**. Users define specific tools in `cast.json` with dedicated JSON schemas and host-side execution templates.

### Key Features
- **Dynamic Tool Registration**: Tools are loaded from `cast.json` and served to agents via MCP's `list_tools`.
- **Structured Validation**: Agent input is validated against the tool's JSON schema using the `jsonschema` crate.
- **Array-Aware Placeholders**: Host commands use placeholders (e.g., `{var}`, `{...array}`) and conditional blocks (`if_present`, `if_true`) for robust mapping to CLI arguments.
- **Security Isolation**:
  - `ApprovedConfig` gate: The server only starts if the configuration is explicitly trusted by the user.
  - Default-Deny Environment: Executed commands have a cleared environment, inheriting only `PATH` and user-specified variables.
  - HTTP/SSE Transport: Standard MCP transport for compatibility with OpenCode agents via `host.docker.internal`.

## Configuration Example

```json
{
  "mcp": {
    "port": 32123,
    "tools": {
      "run_rspec": {
        "description": "Run RSpec tests in the test container",
        "command": "docker",
        "args": [
          "compose", "exec", "test", "bundle", "exec", "rspec",
          { "if_present": "format", "args": ["--format", "{format}"] },
          "--",
          "{...test_paths}"
        ],
        "parameters": {
          "type": "object",
          "properties": {
            "test_paths": { "type": "array", "items": { "type": "string", "pattern": "^spec/.*_spec\\.rb$" } },
            "format": { "type": "string", "enum": ["json", "progress"] }
          },
          "required": ["test_paths"]
        }
      }
    }
  }
}
```

## References

- Implementation Plan: `.mem/feat-mcp/spec/plan.md`
- Execution Roadmap: `.mem/feat-mcp/spec/todo.md`
- `rmcp` Reference: `/home/pl/code/palekiwi-labs/dev-notes/cast/rmcp/`
