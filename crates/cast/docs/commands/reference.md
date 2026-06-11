# Command Reference

Detailed reference for `cast` subcommands.

## `run <agent>`
Starts an agent session.
- `--version <version>`: Use a specific agent version.
- `--no-pull`: Don't try to pull the latest image.

## `build <agent>`
Builds the Docker image for an agent without running it.

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

## `mcp`
Starts the built-in MCP server.
- `--port <port>`: Specify the port to listen on.
- `--hostname <hostname>`: Specify the hostname to bind to.

## `port <agent>`
Prints the deterministic port assigned to the agent for the current workspace.

For implementation details, see [crates/cast/src/commands/](../src/commands/).
