# Configuration Reference

This page lists key configuration fields available in `cast.json`. For the full
schema, see [src/config/schema.rs][schema-src].

## Sandbox Settings

- `memory`: Memory limit for the container (e.g., `"1024m"`).
- `cpus`: CPU limit (e.g., `1.0`).
- `network`: Docker network to use (default: `"bridge"`).
- `forbidden_paths`: List of host paths that should be masked in the sandbox.

## Nix Settings

- `use_flake`: Whether to wrap commands in `nix develop` (default: `false`).
- `use_flake_path`: Specific flake reference to use.
- `nix_volume_name`: Name of the Docker volume for the Nix store.

## Data Volumes

- `volumes_namespace`: Prefix used for automatically named volumes
  (default: `"cast"`).
- `extra_data_volumes`: A map of additional volumes or bind mounts to attach
  to the sandbox.

### Volume Configuration Fields

- `target`: Path inside the container. Supports `~/` for agent home and `./`
  for workspace root.
- `source` (optional): Host path (for `bind`) or volume name (for `volume`).
  Supports `~/` expansion for host paths.
- `type`: Either `"volume"` (default) or `"bind"`.
- `mode`: Either `"rw"` (default) or `"ro"`.

#### Example

```json
{
  "extra_data_volumes": {
    "cargo": {
      "target": "~/.cargo",
      "type": "volume"
    },
    "secrets": {
      "target": "./.secrets",
      "source": "~/.project-secrets",
      "type": "bind",
      "mode": "ro"
    }
  }
}
```

## MCP Settings (`mcp` block)

- `port`: Port for the MCP server (default: `8080`).
- `hostname`: Hostname for the MCP server (default: `"127.0.0.1"`).
- `tools`: Map of tool definitions. See [MCP Configuration][mcp-config]
  for details.

## Agent Versions

- `agent_versions`: Map of agent names to version strings.

[schema-src]: ../../src/config/schema.rs
[mcp-config]: ../mcp/configuration.md
