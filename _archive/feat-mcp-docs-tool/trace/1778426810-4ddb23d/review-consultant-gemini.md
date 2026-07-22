# Review: MCP Documentation Serving Tools

**Status**: ⚠️ Revisions Needed
**Reviewer**: @consultant-gemini

## Executive Summary
The implementation is well-architected and integrates built-in documentation tools cleanly into the existing `McpHandler`. The compile-time embedding strategy ensures portability and zero runtime dependencies for documentation.

## Critical Finding: Documentation Discrepancy
The documentation content in `docs/mcp/configuration.md` is out of sync with the actual implementation schema.
- **Documented**: Uses `host_cmd` as an array (e.g., `["cargo", "test"]`).
- **Actual Implementation**: Expects a `command` string and an `args` array (e.g., `"command": "cargo", "args": ["test"]`).
- **Action**: Update `docs/mcp/configuration.md` to reflect the correct schema fields defined in `src/config/schema.rs`.

## Technical Analysis
- **Interception Pattern**: Idiomatic and clean. Built-in tools are handled before dynamic lookup, avoiding unnecessary validation overhead for static content.
- **Registration**: Correctly prepends built-in tools to `cached_tools` during handler initialization.
- **Error Handling**: Robust use of `McpError::invalid_params` (-32602) for missing arguments or invalid IDs, with helpful guidance in the error messages.
- **Testing**: Good coverage in `handler.rs` for listing, fetching, and error cases.

## Recommendations
1. Fix the schema example in `docs/mcp/configuration.md`.
2. (Minor) Simplify JSON schema construction in `docs.rs` where possible, though the current `json!` macro approach is functional.
