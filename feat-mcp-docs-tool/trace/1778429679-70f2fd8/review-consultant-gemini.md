# Code Review: feat-mcp-docs-tool
**Consultant:** gemini-3-flash-preview
**Date:** 2026-05-10

## Summary
The implementation of built-in MCP documentation tools using `include_dir` is architecturally sound and follows Rust/Nix best practices. The zero-overhead static embedding approach is well-suited for this use case.

## Findings & Recommendations

### 1. Architecture & Performance
- **include_dir**: Confirmed as an excellent choice for zero-overhead static embedding. Its integration with `cargo:rerun-if-changed` ensures build cache consistency.
- **Static Assets**: The decision to use path-based IDs without runtime metadata parsing is endorsed as highly efficient.

### 2. Implementation Improvements (docs.rs)
- **Idiomatic Path Stripping**: Recommended using `if let Some(stripped) = path.strip_suffix(".md")` for more concise and idiomatic Rust.

### 3. Execution Safety & Robustness (exec.rs)
- **Preventing Interactive Hangs**: Recommended explicitly setting `cmd.stdin(Stdio::null())` in `run_command` to prevent tools from hanging if they attempt to read from stdin or trigger interactive prompts.
- **Empty String Handling**: Identified a bug where explicitly passed empty strings might be dropped by the placeholder expansion logic. Recommended tracking whether a placeholder was resolved vs. missing.

### 4. Nix & Build System
- **Source Filtering**: The `flake.nix` changes are safe and correct for Nix sandboxed builds.

## Conclusion
The implementation is solid. The most critical recommendation is the `stdin(Stdio::null())` fix to ensure session stability during tool execution.