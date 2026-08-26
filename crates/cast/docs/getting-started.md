# Getting Started with `cast`

This guide covers the prerequisites and first steps for using `cast`.

## Prerequisites

- **Docker**: `cast` requires Docker to run sandboxes. Ensure your user has
  permission to run Docker commands.
- **Nix**: Recommended for the best experience with flakes and build support.

## Installation

Install `cast` to your Nix profile:

```bash
nix profile add github:palekiwi-labs/cast#cast
```

## First Steps

### 1. Initialize Global Configuration

Bootstrap the global configuration and Nix flake:

```bash
cast config init
```

This creates `~/.config/cast/cast.json` and
`~/.config/cast/nix/flake.nix`. Existing files are never overwritten; if only
one is missing, `cast` creates it and reports that it skipped the other.

### 2. Configure the Project

To add a project devshell, create `cast.json` in the project root with a full
flake reference:

```json
{
  "project_shell": ".#default"
}
```

The generated global configuration already selects
`~/.config/cast/nix#default`, which provides all supported harnesses.

### 3. Approve Configuration

For security, `cast` requires you to approve the configuration for each workspace:

```bash
cast config allow
```

### 4. Run an Agent

Run the `opencode` agent:

```bash
cast run opencode
```

The first time you run an agent, `cast` builds the single, shared dev image
(`localhost/cast:{version}`) automatically. The agent harness is not baked into
the image; it must be provided by the configured global Nix devshell. See
[Flake Integration](nix/flake-integration.md) for details.

## Next Steps

- See the [Command Reference](commands/reference.md) for all available subcommands.
- Learn about [Configuration](config/overview.md) options.
