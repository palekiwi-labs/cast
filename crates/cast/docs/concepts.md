# Core Concepts

`cast` is built on a few key concepts that provide its security and flexibility.

## The Sandbox

A sandbox in `cast` is a Docker container. Every agent runs inside its own
isolated container. The current working directory is typically mounted into the
container, allowing the agent to see and modify your code while remaining
isolated from your host system.

## Agents and Harnesses

`cast` has no knowledge of specific coding agents. Harness binaries are
provisioned by Nix through the devshells you configure (`sandbox_shell`,
`project_shell`), not baked into the image, and their configuration
directories are mounted through `extra_data_volumes` (see
[Configuration Reference](config/reference.md)).

## Nix Integration

`cast` leverages Nix in three ways:

1. **Harness provisioning**: If configured and enabled, `sandbox_shell` is the
   outer devshell and provides the agent binary. The value is a full flake ref
   passed verbatim to Nix.
2. **Project environment**: If configured and enabled, `project_shell` is an
   inner devshell providing project-specific tools. Repositories can define
   both refs, and `cast` performs no implicit flake detection.
3. **Nix Daemon**: `cast` can run a dedicated Nix daemon in a Docker container,
   allowing sandboxes to perform Nix operations safely via a shared volume.

## The Built-in MCP Server

`cast` includes a built-in Model Context Protocol (MCP) server. When you run an
agent, `cast` starts this server, allowing the agent to call tools on the host
(as defined in your `cast.json`) and access project documentation.

## Persistence and Host Access

While sandboxes are isolated, you can configure persistent storage and host
access through `extra_data_volumes`. This allows you to share caches (like
`~/.cargo` or `~/.npm`) between sessions or mount specific host directories
into the sandbox.

## Host Identity (`CAST_HOST_NAME`)

Every sandbox container receives the `CAST_HOST_NAME` environment variable,
set to the host machine's kernel hostname (or the literal string `unknown`
if the hostname cannot be resolved). This lets diagnostics or telemetry
collected inside the container be grouped by the host that launched it.

The value is whatever `gethostname(2)` returns — typically the short
hostname, but it may be an FQDN depending on host configuration. No DNS
resolution is performed; the value is stable for a given host.

For implementation details, refer to the source code:

- Sandbox logic: [src/dev/][sandbox-src]
- Host resolution: [src/host/][host-src]
- MCP implementation: [src/mcp/][mcp-src]

[sandbox-src]: ../src/dev/
[host-src]: ../src/host/
[mcp-src]: ../src/mcp/
