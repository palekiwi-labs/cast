# MCP Documentation Serving Tools

## Goal
Implement two built-in MCP tools (`list_cast_documentation` and `fetch_cast_documentation`) to serve documentation embedded in the `cast` binary to AI agents.

## Design: Static Documentation Discovery

### 1. Automatic Embedding (`include_dir`)
- Use the `include_dir` crate to embed the entire `docs/` directory as a static tree in the binary at compile time.
- This ensures zero runtime initialization cost and zero manual maintenance when adding new files.

### 2. Path-based Identity
- **ID Generation**: The relative path to the file (e.g., `mcp/configuration`) will be used as the stable ID.
- **No Metadata Extraction**: Titles and descriptions are omitted to keep the implementation purely static and lightweight. The LLM is expected to infer the purpose of a document from its path-based ID.

### 3. MCP Tools
- **`list_cast_documentation`**: Iterates over the embedded files and returns a flat list of available IDs (paths).
- **`fetch_cast_documentation`**: Parameter `id` (string). Appends `.md` and retrieves the raw content from the embedded tree.

### 4. Build System (Nix)
- The Nix source filter must be updated to explicitly include the `docs/` directory and `.md` files, as they are required at compile time for the `include_dir!` macro.

### 5. Error Handling
- **Missing Args**: Return error if `id` is omitted when fetching.
- **Not Found**: Return error if the requested entry doesn't exist, advising the use of the list tool.
