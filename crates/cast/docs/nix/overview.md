# Nix Overview

`cast` provides deep integration with Nix to ensure your agent sandboxes have
access to the same reproducible environment as your host.

## Modes of Integration

### 1. Harness Provisioning (global devshell)

The dev image ships with no agent binaries baked in. Set `global_shell` to a
full flake reference whose devshell provides the requested harness. Running
`cast config init` creates a global flake and configures
`~/.config/cast/nix#default`, which provides all supported harnesses. Repositories
can instead select any other ref, including a repository-defined shell.

### 2. Explicit Shell Layers

`global_shell` and `project_shell` are symmetric, explicit flake references.
When set and enabled, `cast` passes each ref verbatim to `nix develop <ref> -c`.
There is no flake-file detection or agent-name fallback. Relative project refs
resolve from the mounted workspace inside the container.

This wrapping also applies to `cast shell`, so an interactive shell starts
inside the devshell by default. Use `cast shell --raw <agent>` to bypass it.

### 3. Nix Daemon Volume

`cast` can run a dedicated Nix daemon in a Docker container.

- The Nix store is shared via a Docker volume (default: `cast-nix`).
- The daemon container has `rw` access to the store.
- Agent sandboxes have `ro` access to the store.
- Communication happens over a Unix socket.

This allows agents to run Nix commands (like `nix build`) inside the sandbox
without needing Nix installed in the sandbox image itself.

For more details, see:

- [Nix Daemon](daemon.md)
- [Flake Integration](flake-integration.md)
