# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-07-24

### Changed

- Nix-native harness provisioning: harnesses are no longer baked into Docker
  images. They are now provided exclusively by named devShells of the global
  cast flake (`~/.config/cast/nix/flake.nix`), selected at run time.
- A single, harness-free dev image (`localhost/cast:<version>`) replaces the
  three per-agent images and the content-hash image tag.
- `cast run <agent>` selects a named global devShell (default: the agent name),
  overridable via the new `global_shell` config field.

### Added

- Embedded global-flake template with per-harness devShells (opencode, pi,
  claudecode) plus a `universal` shell and the `cache.numtide.com` binary
  cache. Auto-scaffolded on first `cast run` when no global flake is present.
- `templates.global` flake output for `nix flake init -t`.
- `accept-flake-config = true` in the dev container's system nix.conf so
  harness substituter fetches succeed non-interactively.

### Removed

- `agent_versions` config field (harness versions are now pinned by the global
  flake's `flake.lock`). Leftover keys in `cast.json` are silently ignored.
- `universal_container` config field; universal mounts/volumes are now the
  unconditional default.
- `version_cache_ttl_hours` config field (the version-resolution machinery it
  fed has been removed).

## [0.1.0] - 2026-06-11

### Added

- Initial release of `cast` - coding agent sandbox tool.
- Built-in MCP server for tool execution in sandboxes.
- Support for OpenCode, ClaudeCode, and Pi agents.
- Nix daemon integration for build support.
- `cast-mcp-client` for interacting with MCP servers.
