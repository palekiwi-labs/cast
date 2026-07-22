# Project Log

## [876aff5] Restructure into Workspace and Complete MCP Client Call

Restructured the project into a Cargo workspace to decouple the host-side sandbox manager from the container-side MCP client.

Key achievements:
- Created `crates/cast` for the main CLI and MCP Server.
- Created `crates/cast-mcp-client` as a dual library/binary for lightweight container use.
- Implemented the `call` subcommand with full JSON and stdin support.
- Updated `flake.nix` to support independent building of the two binaries.
- Verified all integration tests pass with the new workspace structure.

Decisions:
- Used path dependencies for internal code reuse.
- Maintained `cast mcp` subcommands as thin wrappers over the `cast-mcp-client` library.
- Relocated `assets/` into `crates/cast/` as they are host-specific.

- **Found:** include_str! paths need adjustment when source moves deeper into the tree
- **Found:** Integration tests need to import the client from the new workspace member crate
- **Decided:** Use Cargo Workspace for decoupling host and container tools
- **Decided:** Expose cast-mcp-client as both a library and a binary
- **Decided:** Update flake.nix to prevent closure bloat in containers

## [3391ffe] Strict Decoupling of MCP Client and Server

Strictly decoupled the MCP client from the main cast tool.

Key changes:
- Removed client subcommands (list, describe, call) from the `cast` CLI.
- Migrated client integration tests to the `cast-mcp-client` crate.
- Cleaned up `cast` dependencies to remove any reference to the client crate.
- Verified that `cast` remains functional for its host-side duties (server start, sandbox management) while `cast-mcp-client` becomes the definitive tool for container-side tool discovery and execution.

Decision:
- Keep `cast mcp` in the main CLI but limit it to server-side operations to maintain a clear responsibility split: `cast` manages the host/server, `cast-mcp-client` manages the container/client.

- **Found:** Integration tests previously relying on 'cast' binary now correctly target 'cast-mcp-client'.
- **Decided:** Remove client wrappers from cast CLI entirely to prevent dependency leakage and maintain strict responsibility separation.

