# Environment Overrides

Every configuration field in `cast` can be overridden by environment variables.

## Naming Convention

- Prefix: `CAST_`
- Case: ALL_CAPS
- Nesting: Double underscore `__`

## Examples

| Config Field   | Env Variable         |
| -------------- | -------------------- |
| `memory`       | `CAST_MEMORY`        |
| `cpus`         | `CAST_CPUS`          |
| `mcp.port`     | `CAST_MCP__PORT`     |
| `mcp.hostname` | `CAST_MCP__HOSTNAME` |
| `use_flake`    | `CAST_USE_FLAKE`     |

## Specialized Env Vars

- `CAST_LOG_DIR`: Directory where daily rolling logs are stored.
- `CAST_DATA_DIR`: Directory where `approved_configs.json` and other state are
  stored.
