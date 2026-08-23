# Agents

`cast` supports multiple coding agents through a pluggable harness system.

## Supported Agents

- **OpenCode**: A versatile open-source coding agent.
- **ClaudeCode**: Integration for Anthropic's Claude-based coding tools.
- **Pi**: A specialized agent harness.

## Nix-native harness provisioning

Harness binaries are **not** baked into the Docker image. There is a single,
harness-free dev image (`localhost/cast:{version}`); every agent runs in it.
The harness itself is provided by a named devShell of the global cast flake
(`~/.config/cast/nix/flake.nix`), selected at run time.

`cast run <agent>` enters the devShell named after the agent by default
(e.g. `opencode`). Override the shell with the `global_shell` config field —
for example, point every agent at a `universal` shell that exposes all
harnesses at once. See [Flake Integration](nix/flake-integration.md) for the
shell-selection mechanics and the shipped template.

Because provisioning lives in the flake, harness **versions are pinned by the
global flake's `flake.lock`**, not by `cast.json`.

## The `Agent` Trait

The system is extensible via the `Agent` trait. Since harnesses come from the
global devShell, the trait defines only the program-specific layer on top of
the shared image:
1. Config-directory bind mounts (`config_mount_args`)
2. Host preparation (creating directories) via `prepare_host`
3. Command wrapping (nested `nix develop` shell selection) via `build_command`

The trait no longer handles version resolution or image building — those are
gone with the nix-native model. For the full trait definition, see
[src/dev/agent.rs][agent-trait].

## Adding New Agents

Adding a new agent involves implementing the `Agent` trait, registering the
harness in the CLI, and adding a devShell of the same name to the global flake
template. The source code for existing harnesses provides the best template:
- OpenCode: [src/dev/opencode/mod.rs][opencode-harness]
- ClaudeCode: [src/dev/claudecode/mod.rs][claudecode-harness]
- Pi: [src/dev/pi/mod.rs][pi-harness]

[agent-trait]: ../src/dev/agent.rs
[opencode-harness]: ../src/dev/opencode/mod.rs
[claudecode-harness]: ../src/dev/claudecode/mod.rs
[pi-harness]: ../src/dev/pi/mod.rs
