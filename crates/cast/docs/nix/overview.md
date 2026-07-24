# Nix Overview

`cast` provides deep integration with Nix to ensure your agent sandboxes have
access to the same reproducible environment as your host.

## Modes of Integration

### 0. Harness Provisioning (global devShell)

The agent binary itself comes from a named devShell of the global cast flake
at `~/.config/cast/nix/flake.nix`. `cast run <agent>` enters the devShell named
after the agent by default; the `global_shell` config field overrides this
(for example, to select a `universal` shell providing every harness). The dev
image ships with no harness baked in. See [Flake Integration](flake-integration.md)
for shell selection, the shipped template, and auto-scaffolding.

### 1. Flake Wrapping

When `use_flake` is set to `true`, `cast` detects your project's `flake.nix`. It
wraps the agent's entrypoint command in `nix develop -c`. This means the agent
sees exactly the same `PATH` and environment variables as if you had run `nix
develop` on your host.

This wrapping also applies to `cast shell`, so an interactive shell starts
inside the devshell by default. Use `cast shell --raw <agent>` to bypass it.

### 2. Nix Daemon Volume

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
