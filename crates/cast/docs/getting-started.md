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

### 1. Initialize Configuration

`cast` looks for a `cast.json` file in your project root.
You can create a minimal one:

```json
{
  "use_flake": true
}
```

### 2. Approve Configuration

For security, `cast` requires you to approve the configuration for each workspace:

```bash
cast config allow
```

### 3. Run an Agent

Run the `opencode` agent:

```bash
cast run opencode
```

The first time you run an agent, `cast` builds the single, shared dev image
(`localhost/cast:{version}`) automatically. The agent harness is not baked into
the image — it is provided by a global Nix devShell. If you have no global flake
yet (`~/.config/cast/nix/flake.nix`), `cast` scaffolds one from a shipped
template so the first run works with no manual setup. See
[Flake Integration](nix/flake-integration.md) for details.

## Next Steps

- See the [Command Reference](commands/reference.md) for all available subcommands.
- Learn about [Configuration](config/overview.md) options.
