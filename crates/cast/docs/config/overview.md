# Configuration Overview

`cast` uses a hierarchical configuration system.

## Configuration Files

1. **Global Config**: `~/.config/cast/cast.json`
2. **Project Config**: `./cast.json` (at the workspace root)
3. **Flat MCP Config**: `./cast-mcp.json` (merged into the `mcp` section)

## Loading Precedence

Higher priority overrides lower priority:
1. **Environment Variables** (`CAST_*`)
2. **Flat MCP Config** (`./cast-mcp.json`)
3. **Project Config** (`./cast.json`)
4. **Global Config** (`~/.config/cast/cast.json`)
5. **Hardcoded Defaults**

## Environment Variables

Use `CAST_` prefix. Nested fields use double underscores:
- `CAST_MEMORY` → `memory`
- `CAST_MCP__PORT` → `mcp.port`

See the [Configuration Reference](reference.md) for available fields.
