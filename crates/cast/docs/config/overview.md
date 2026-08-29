# Configuration Overview

`cast` uses a hierarchical configuration system.

## Configuration Files

1. **Global Config**: `~/.config/cast/cast.json`
2. **Project Config**: `./cast.json` (at the workspace root)
3. **Local Project Config**: `./cast.local.json` (personal overrides)
4. **Flat MCP Config**: `./cast-mcp.json` (merged into the `mcp` section)

## Loading Precedence

Higher priority overrides lower priority:

1. **Environment Variables** (`CAST_*`)
2. **Flat MCP Config** (`./cast-mcp.json`)
3. **Local Project Config** (`./cast.local.json`)
4. **Project Config** (`./cast.json`)
5. **Global Config** (`~/.config/cast/cast.json`)
6. **Hardcoded Defaults**

## Local Overrides

`cast.local.json` uses the same schema as `cast.json`. Use it for settings that
are specific to your machine, such as `extra_data_volumes` bind mounts that
point to local directories. Keys omitted from the local file fall through to
`cast.json`; arrays replace lower-precedence arrays rather than concatenate.

Add the file to the project's `.gitignore` before creating it so personal
settings are not committed:

```gitignore
cast.local.json
```

The approval hash covers the fully merged configuration. Adding or changing
`cast.local.json` after running `cast config allow` therefore requires review
with `cast config diff` and re-approval with `cast config allow`.

## Environment Variables

Use `CAST_` prefix. Nested fields use double underscores:

- `CAST_MEMORY` → `memory`
- `CAST_MCP__PORT` → `mcp.port`

See the [Configuration Reference](reference.md) for available fields.
