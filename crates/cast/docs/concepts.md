# Core Concepts

`cast` is built on a few key concepts that provide its security and flexibility.

## The Sandbox

A sandbox in `cast` is a Docker container. Every agent runs inside its own
isolated container. The current working directory is typically mounted into the
container, allowing the agent to see and modify your code while remaining
isolated from your host system.

## Agents and Harnesses

An **Agent** is a specific coding tool (like OpenCode, ClaudeCode, or Pi).
A **Harness** is the implementation of the `Agent` trait in `cast` that knows
how to build the Docker image and run the agent binary with the correct flags
and environment.

## Nix Integration

`cast` leverages Nix in two ways:

1. **Flake Wrapping**: If `use_flake` is enabled, `cast` wraps the agent's
   execution in a `nix develop` shell, providing the agent with the exact tools
   defined in your project's `flake.nix`.
2. **Nix Daemon**: `cast` can run a dedicated Nix daemon in a Docker container,
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
