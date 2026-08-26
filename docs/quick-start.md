# Quick Start Guide

This guide will help you get `cast` up and running in 5 minutes using Nix.

## Prerequisites

- **Docker**: `cast` uses Docker to run sandboxes. Ensure Docker is installed
  and the daemon is running.
- **Nix**: `cast` is distributed via Nix flakes.

## 1. Install `cast`

Install `cast` to your Nix profile:

```bash
nix profile add github:palekiwi-labs/cast#cast
```

## 2. Initialize the global environment

Create the default global configuration and Nix flake:

```bash
cast config init
```

Existing files are never overwritten.

## 3. Run your first agent

To run an agent, you first need to approve the project configuration.
In your project directory, run:

```bash
cast config allow
```

Then, run the `opencode` agent:

```bash
cast run opencode
```

This will:

1. Build the shared, harness-free sandbox image (first run only).
2. Start a Docker container with the current directory mounted.
3. Enter the explicitly configured `default` global devshell, which provides
   the supported agent harnesses.
4. Launch the `opencode` agent inside it.

## Next Steps

- Explore the [cast crate documentation](../crates/cast/docs/README.md) for
  detailed configuration and usage.
- Learn how to use the [MCP server](../crates/cast/docs/mcp/overview.md).
