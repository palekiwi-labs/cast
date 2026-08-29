# Agents

`cast` supports multiple coding agents through a pluggable harness system.

## Supported Agents

- **OpenCode**: A versatile open-source coding agent.
- **ClaudeCode**: Integration for Anthropic's Claude-based coding tools.
- **Pi**: A specialized agent harness.

## Nix-native harness provisioning

Harness binaries are **not** baked into the Docker image. There is a single,
harness-free dev image (`localhost/cast:{version}`); every agent runs in it.
The harness itself is provided by the devshell referenced by `sandbox_shell`,
selected at run time.

`sandbox_shell` is a full Nix flake reference, such as
`~/.config/cast/nix#default` or `.#ai`. There is no default based on the agent
name: if the field is unset, no sandbox devshell is entered. The template created
by `cast config init` has a `default` shell containing all supported harnesses
and optional per-harness shells. See
[Flake Integration](nix/flake-integration.md) for the selection mechanics.

Because provisioning lives in the flake, harness **versions are pinned by the
selected flake's `flake.lock`**, not by `cast.json`.

## The `Agent` Trait

The system is extensible via the `Agent` trait. Since harnesses come from the
sandbox devShell, the trait defines only the program-specific layer on top of
the shared image:
1. Config-directory bind mounts (`config_mount_args`)
2. Host preparation (creating directories) via `prepare_host`
3. Command wrapping (nested `nix develop` shell selection) via `build_command`

The trait no longer handles version resolution or image building — those are
gone with the nix-native model. For the full trait definition, see
[src/dev/agent.rs][agent-trait].

## Adding New Agents

Adding a new agent involves implementing the `Agent` trait and registering the
harness in the CLI. Users must select a `sandbox_shell` whose packages provide
the corresponding binary; shell names do not need to match agent names. The
source code for existing harnesses provides the best template:
- OpenCode: [src/dev/opencode/mod.rs][opencode-harness]
- ClaudeCode: [src/dev/claudecode/mod.rs][claudecode-harness]
- Pi: [src/dev/pi/mod.rs][pi-harness]

[agent-trait]: ../src/dev/agent.rs
[opencode-harness]: ../src/dev/opencode/mod.rs
[claudecode-harness]: ../src/dev/claudecode/mod.rs
[pi-harness]: ../src/dev/pi/mod.rs
