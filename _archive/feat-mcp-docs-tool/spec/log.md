# Project Log

## [fe7f40c] Implemented built-in MCP documentation tools

- **Found:** Successfully implemented list_cast_documentation and fetch_cast_documentation.
- **Decided:** Used compile-time embedding (include_str!) for documentation portability.
- **Decided:** Registered built-in tools in McpHandler bypassing dynamic config lookup.

## [4ddb23d] Fixed formatting and amended commit

- **Found:** Applied cargo fmt to new and modified files.
- **Decided:** Kept the documentation serving feature in a single atomic commit.

## [4ddb23d] Pivot to static include_dir approach

- **Found:** Linear search on small static sets is faster and simpler
- **Decided:** Use include_dir instead of rust-embed/OnceLock/HashMap

## [7cfacdd] Implemented static documentation serving

Completed the implementation of list_cast_documentation and fetch_cast_documentation tools. The implementation is purely static, with zero runtime initialization cost, and is fully compatible with Nix builds (after the source filter update).

- **Found:** include_dir provides a cleaner static-only solution than rust-embed
- **Decided:** Path-based IDs are sufficient for AI agents to discover documentation

## [28f4486] Fixed potential interactive hangs in MCP tools

Implemented the safety harness in exec.rs by explicitly setting stdin to null. Also performed stylistic cleanup in docs.rs as recommended in the code review.

- **Found:** Subprocess inheritance of stdin was verified as a critical hang risk
- **Decided:** Set Stdio::null() for tool stdin to ensure EOF and prevent deadlocks

## [28f4486] Research complete: MCP configuration

- **Found:** Mapped McpToolConfig and ArgTemplate structure
- **Found:** Verified placeholder expansion for {name} and {...name}
- **Found:** Identified conditional block logic for if_present and if_true
- **Found:** Confirmed tool execution flow and built-in routing

## [28f4486] Research Findings: MCP Configuration Details

- **Found:** The MCP server uses a dynamic routing system that maps tool calls to shell commands.
- **Found:** Argument templates support literals, placeholders {name}, and the spread operator {...name}.
- **Found:** Conditional blocks (if_present, if_true) allow for flexible CLI argument construction.
- **Found:** Environment variables can be inherited or explicitly set per tool.
- **Found:** Built-in documentation tools are available by default.

## [c82a473] Research complete: MCP Configuration Verification

- **Found:** hostname configuration support in McpConfig
- **Found:** Implicit inheritance of PATH and TMPDIR for tool execution

## [d2576d8] Resolved Workspace Merge Conflicts and Restored Doc Tools

Successfully resolved merge conflicts between 'feat/mcp-docs-tool' and the workspace-restructured 'dev' branch.

Key actions:
- Converted root Cargo.toml to a Workspace manifest.
- Moved docs/ directory to crates/cast/docs/ to satisfy include_dir! macro within the cast crate.
- Merged Nix source filter logic in flake.nix to include documentation in the build sandbox.
- Added include_dir dependency to crates/cast/Cargo.toml.
- Verified doc tools via 'cargo test -p cast documentation'.

The project is now a clean workspace with built-in MCP documentation serving functional.

- **Found:** include_dir macro expects paths relative to CARGO_MANIFEST_DIR
- **Found:** Nix build requires explicit inclusion of non-source files via filter when cleanSource is used
- **Decided:** Use Cargo Workspace for all future developments
- **Decided:** Keep documentation within the crate that serves it (crates/cast/docs)

