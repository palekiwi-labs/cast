# Trace: MCP Server URL Resolution Redesign

## Problem Statement
The initial implementation of `resolve_server_url` was overengineered and fragile. It attempted to reread `cast.json` inside the container, which:
1. Violated security/isolation principles (container shouldn't see host execution templates).
2. Duplicated complex configuration and "approval" logic.
3. Relied on brittle container-detection heuristics.

## Core Insight
Discovery belongs at the **Orchestration Layer**, not the **Consumer Layer**. Since `cast run` launches the container on the host (where it has full access to the approved config), it should calculate the URL once and inject it into the container.

## Chosen Strategy: Active Environment Injection
Resolution is split across the host-container boundary:

### 1. Host-Side (Orchestrator)
Inside `cast run`:
- Load `ApprovedConfig` (host-side).
- Resolve `mcp.hostname` and `mcp.port`.
- **Boundary Translation**: If hostname is a loopback (`127.0.0.1`, `localhost`) or wildcard (`0.0.0.0`), rewrite it to `host.docker.internal` (the Docker gateway).
- **Injection**: Pass `-e CAST_MCP_URL=http://host.docker.internal:PORT/mcp` to the `docker run` command.

### 2. Container-Side (Consumer)
Inside `cast mcp call`:
- Use a "Dumb" fallthrough:
  1. `--url` CLI flag (explicit override).
  2. `CAST_MCP_URL` environment variable (injected at launch).
  3. Default fallback: `http://host.docker.internal:8080/mcp`.
  4. Failure: Exit with a clear error message.

## Benefits
- **Zero File IO**: No need to mount `cast.json` into the container.
- **Single Source of Truth**: Host config remains the absolute authority.
- **Robustness**: Port/Hostname changes on the host propagate automatically to the container at launch.
- **Simplicity**: Deletes ~50 lines of complex detection and parsing code.
- **Inheritance**: Works out-of-the-box for `cast shell` (exec) sessions.

## Decision Audit
- **Decided**: Use Option 1 (Environment Injection) as the primary discovery mechanism.
- **Decided**: Abandon manual config parsing and "approval" logic inside the container.
- **Decided**: Maintain a simple fallthrough in the client for local (host-side) interactive use.
