# cast

`cast` is a coding agent sandbox tool that orchestrates Docker-based sandboxes
for coding agents like OpenCode, ClaudeCode, and Pi. It provides a secure,
reproducible environment with Nix build support and a built-in MCP (Model
Context Protocol) server.

## Installation (Nix)

The recommended way to use `cast` is via Nix.

### Run directly
```bash
nix run github:palekiwi-labs/cast#cast -- --help
```

### Install to profile
```bash
nix profile install github:palekiwi-labs/cast#cast
```

## Documentation

For detailed documentation, please refer to the [docs/](docs/) directory.

- [Getting Started](docs/quick-start.md)
- [Crate Documentation (cast)](crates/cast/docs/README.md)
- [Crate Documentation (cast-mcp-client)](crates/cast-mcp-client/docs/README.md)
