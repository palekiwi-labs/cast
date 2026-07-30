# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- The nix daemon's binary caches are now provisioned daemon-side from
  `~/.config/cast/cast.json`. The first `cast run`/`cast exec` on a fresh host
  seeds a default `cast.json` with the numtide cache so harnesses fetch
  prebuilt rather than building from source. An existing `cast.json` is never
  overwritten.

### Changed

- **Security:** the nix daemon's `trusted-users` is tightened from `root *` to
  `root` (with `allowed-users = *`). The non-trusted dev container user can
  still connect to the daemon but can no longer override substituters, add
  trusted keys, or import unsigned paths into the shared `/nix` store.
- The dev container sets `accept-flake-config = false`. Under the hardened
  daemon the non-trusted user's flake-forwarded settings are rejected anyway;
  caches are provisioned via `cast.json` instead.
- **Breaking:** `cast build <agent>` is replaced by a single `cast build`. The
  three per-agent build subcommands built the identical shared dev image, so
  the agent argument was removed. `--force` and `--no-cache` are unchanged.
- **Breaking:** `cast build`'s `--base` flag is renamed to `--nix-daemon`,
  matching the `cast nix-daemon` command it triggers. The dev image is not
  built from the nix-daemon image, so "base" was a misnomer.

### Fixed

- Spurious `ignoring untrusted substituter 'https://cache.nixos.org/'` warnings
  from the nix daemon. The daemon's `trusted-substituters` now lists every
  substituter in both spellings (with and without a trailing slash), since nix
  compares client-forwarded substituters as exact strings and only compensates
  for a *missing* slash, never an extra one.

### Removed

- Orphaned `ureq` dependency, left over after the nix-native pivot deleted the
  harness version fetcher.

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
