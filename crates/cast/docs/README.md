# cast Documentation

The `cast` crate is the core of the coding agent sandbox tool. It manages
Docker-based sandboxes, provides Nix build support, and runs a built-in MCP
server.

## Sections

- **[Getting Started](getting-started.md)**: Prerequisites and first-run guide.
- **[Concepts](concepts.md)**: Mental model of sandboxes, Docker, and Nix
  integration.
- **[Command Reference](commands/reference.md)**: Detailed guide to all `cast`
  subcommands.
- **[Agents](agents.md)**: Supported agents and the `Agent` trait.
- **[Configuration](config/overview.md)**: Loading precedence and field
  reference.
- **[Nix Integration](nix/overview.md)**: Flake wrapping and the Nix daemon.
- **[MCP Server](mcp/overview.md)**: Built-in Model Context Protocol server.

## For Developers

- See [src/lib.rs](../src/lib.rs) for the module overview.
- The `Agent` trait is defined in [src/dev/agent.rs](../src/dev/agent.rs).
