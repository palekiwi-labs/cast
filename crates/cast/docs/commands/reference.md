# Command Reference

Detailed reference for `cast` subcommands.

## `run <agent>`
Starts an agent session.
- `[extra_args]`: Extra arguments to pass directly to the agent binary.

## `build <agent>`
Builds the Docker image for an agent without running it.
- `--base`: Also build the Nix daemon base image.
- `--force`, `-f`: Force rebuild even if image already exists.
- `--no-cache`: Do not use Docker cache.

## `shell <agent>`
Starts an interactive shell inside the agent's sandbox.

## `config`
Manages project configuration and approvals.
- `config show`: Display the current merged configuration.
- `config allow`: Approve the current project configuration.
- `config deny`: Remove approval for the project configuration.
- `config diff`: Show the diff between the current and approved configuration.

## `nix-daemon`
Manages the containerized Nix daemon.
- `nix-daemon build`: Build the nix-daemon image.
- `nix-daemon start`: Start the nix-daemon container.
- `nix-daemon stop`: Stop the nix-daemon container.
- `nix-daemon shell`: Open a shell in the nix-daemon container.

## `mcp start`
Starts the built-in MCP server.
- `--port <port>`: Specify the port to listen on (overrides cast.json).
- `--host <host>`: Specify the host to bind to (overrides cast.json).

## `port <agent>`
Prints the deterministic port assigned to the agent for the current workspace.

For implementation details, see [crates/cast/src/commands/](../src/commands/).
