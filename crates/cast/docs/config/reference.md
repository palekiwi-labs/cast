# Configuration Reference

This page lists key configuration fields available in `cast.json`. For the full schema, see [crates/cast/src/config/schema.rs](../../src/config/schema.rs).

## Sandbox Settings

- `memory`: Memory limit for the container (e.g., `"1024m"`).
- `cpus`: CPU limit (e.g., `1.0`).
- `network`: Docker network to use (default: `"bridge"`).
- `forbidden_paths`: List of host paths that should be masked in the sandbox.

## Nix Settings

- `use_flake`: Whether to wrap commands in `nix develop` (default: `false`).
- `use_flake_path`: Specific flake reference to use.
- `nix_volume_name`: Name of the Docker volume for the Nix store.

## MCP Settings (`mcp` block)

- `port`: Port for the MCP server (default: `8080`).
- `hostname`: Hostname for the MCP server (default: `"127.0.0.1"`).
- `tools`: Map of tool definitions. See [MCP Configuration](../mcp/configuration.md) for details.

## Agent Versions

- `agent_versions`: Map of agent names to version strings.
