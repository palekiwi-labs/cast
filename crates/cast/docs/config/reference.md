# Configuration Reference

This page lists key configuration fields available in both `cast.json` and
`cast.local.json`. For the full schema, see
[src/config/schema.rs][schema-src].

## Sandbox Settings

- `memory`: Memory limit for the container (e.g., `"1024m"`).
- `cpus`: CPU limit (e.g., `1.0`).
- `network`: Docker network to use (default: `"bridge"`).
- `forbidden_paths`: List of host paths that should be masked in the sandbox.
- `env_passthrough`: Base list of host environment variable *names* to forward
  into the sandbox (intended for global config). Values are read from the host
  at run time and never stored. See [Environment Overrides][env-overrides].
- `extra_env_passthrough`: Additional list of host environment variable *names*
  to forward into the sandbox (intended for project config). Values are read
  from the host at run time and never stored. See
  [Environment Overrides][env-overrides].

## Nix Settings

- `sandbox_shell`: Full flake reference for the outer harness layer, such as
  `~/.config/cast/nix#default`, `.#ai`, or `github:org/repo#shell`. Unset means
  no sandbox layer. A leading `~/` resolves against the container user's home;
  other refs are passed verbatim to `nix develop`.
- `project_shell`: Full flake reference for the inner project layer, such as
  `.#default`. Unset means no project layer. Relative refs resolve from the
  workspace inside the container.
- `use_sandbox_shell`: Whether to enable the configured sandbox layer (default:
  `true`).
- `use_project_shell`: Whether to enable the configured project layer (default:
  `true`).
- `nix_volume_name`: Name of the Docker volume for the Nix store.

The removed `global_shell`, `use_flake`, and `use_flake_path` keys are silently
ignored. Replace a bare `global_shell` name with a complete `sandbox_shell` ref;
migrate the other keys to `project_shell`. See
[Flake Integration][flake-integration]. The corresponding
`CAST_GLOBAL_SHELL` override was also removed.

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

Harness versions are no longer configured in `cast.json`. They are pinned by
the selected flake's `flake.lock` (see [Flake Integration][flake-integration]).
A legacy `agent_versions` key is silently ignored if present.

[schema-src]: ../../src/config/schema.rs
[mcp-config]: ../mcp/configuration.md
[env-overrides]: env-overrides.md
[flake-integration]: ../nix/flake-integration.md
