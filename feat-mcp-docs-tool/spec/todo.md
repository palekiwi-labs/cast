# TODO: MCP Documentation Tools

## Scope
Implement built-in MCP tools to serve embedded `cast` documentation.

## Tasks

- [x] 1. Add `include_dir` to `Cargo.toml`.
- [x] 2. Update Nix source filter in `flake.nix` to include `docs/` directory.
- [x] 3. Refactor `src/commands/mcp/docs.rs` to use `include_dir` and remove `DocEntry` metadata.
- [x] 4. Update `list_cast_documentation` tool to dynamically list paths from the embedded directory.
- [x] 5. Update `fetch_cast_documentation` tool to lookup files by path.
- [x] 6. Verify implementation with `cargo check` and `nix build`.
- [x] 7. Add `mem log` marking completion of the feature.
