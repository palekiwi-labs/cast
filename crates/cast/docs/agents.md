# Agents

`cast` supports multiple coding agents through a pluggable harness system.

## Supported Agents

- **OpenCode**: A versatile open-source coding agent.
- **ClaudeCode**: Integration for Anthropic's Claude-based coding tools.
- **Pi**: A specialized agent harness.

## The `Agent` Trait

The system is extensible via the `Agent` trait. This trait defines the
lifecycle of an agent session:
1. Version resolution
2. Image building (from an embedded Dockerfile)
3. Host preparation (creating directories/volumes)
4. Docker argument generation
5. Command wrapping (including Nix support)

For the full trait definition, see [src/dev/agent.rs][agent-trait].

## Adding New Agents

Adding a new agent involves implementing the `Agent` trait and registering the
new harness in the CLI. The source code for existing harnesses provides the
best template:
- OpenCode: [src/dev/opencode/mod.rs][opencode-harness]
- ClaudeCode: [src/dev/claudecode/mod.rs][claudecode-harness]
- Pi: [src/dev/pi/mod.rs][pi-harness]

[agent-trait]: ../src/dev/agent.rs
[opencode-harness]: ../src/dev/opencode/mod.rs
[claudecode-harness]: ../src/dev/claudecode/mod.rs
[pi-harness]: ../src/dev/pi/mod.rs
