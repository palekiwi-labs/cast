# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `cast config init`, a flagless global bootstrap command that creates the
  default `~/.config/cast/cast.json` and `~/.config/cast/nix/flake.nix`. It
  never overwrites existing files and can initialize either missing file
  independently.
- Explicit `sandbox_shell` and `project_shell` full flake references, with
  `use_sandbox_shell` and `use_project_shell` switches. Environment overrides
  are available as `CAST_SANDBOX_SHELL`, `CAST_PROJECT_SHELL`,
  `CAST_USE_SANDBOX_SHELL`, and `CAST_USE_PROJECT_SHELL`.
- Optional `cast.local.json` project configuration for untracked,
  machine-specific overrides of `cast.json`.
- `env_passthrough` and `extra_env_passthrough` configuration keys in
  `cast.json` to allowlist host environment variable names for forwarding into
  containers without storing or leaking secret values to disk, approval
  snapshots, or logs.
- `CAST_ENV_PASSTHROUGH` and `CAST_EXTRA_ENV_PASSTHROUGH` environment variable
  overrides using bracketed list syntax (e.g. `'[GH_TOKEN]'`).
- The nix daemon's binary caches are now provisioned daemon-side from
  `~/.config/cast/cast.json`. `cast config init` seeds the default with the
  numtide cache so harnesses fetch prebuilt rather than building from source.

### Changed

- **Breaking:** Shell selection is now fully explicit. `cast` passes configured
  refs verbatim to `nix develop`; it no longer detects project or user flakes,
  derives shell fragments from agent names, or assumes a global flake path.
- The shipped global flake's `default` devshell now contains all supported
  harnesses. The former `universal` shell name has been removed; per-harness
  shells remain available.
- `~/.config/cast/nix` is mounted whenever the directory exists, without
  requiring a `flake.nix` file.
- Adding `env_passthrough` and `extra_env_passthrough` to the configuration
  schema alters the serialized JSON hash of `Config`. Existing approved project
  configurations will report `ApprovalStatus::Changed` once on upgrade and
  require `cast config allow`.
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

- Development and execution containers now run with `--init`, so `docker-init`
  (tini) is PID 1 and reaps orphaned children. Agents and shells spawn deep
  process trees whose orphans previously lingered as zombies and accumulated
  against `--pids-limit` until the container could no longer fork. tini also
  forwards signals, so Ctrl+C still reaches nested processes.
- The global flake template no longer declares a `nixConfig` cache block. It
  made nix prompt for approval on every devshell entry, a prompt the
  non-trusted dev user cannot usefully answer since the daemon rejects the
  settings either way. Caches ship in the seeded `cast.json` instead. Existing
  global flakes are never overwritten — delete the block by hand, and remove
  any stale `~/.local/share/nix/trusted-settings.json`, to stop the prompt.
- Spurious `ignoring untrusted substituter 'https://cache.nixos.org/'` warnings
  from the nix daemon. The daemon's `trusted-substituters` now lists every
  substituter in both spellings (with and without a trailing slash), since nix
  compares client-forwarded substituters as exact strings and only compensates
  for a *missing* slash, never an extra one.

### Removed

- **Breaking:** `global_shell` and its `CAST_GLOBAL_SHELL` override. Legacy
  keys are silently ignored. **Migration:** replace a bare shell name with a
  complete `sandbox_shell` ref, such as `~/.config/cast/nix#default`.
- **Breaking:** `use_flake` and `use_flake_path`, including the
  `CAST_USE_FLAKE` and `CAST_USE_FLAKE_PATH` overrides. Legacy keys are
  silently ignored. **Migration:** set `project_shell` to a complete ref such
  as `.#default`; use `use_project_shell: false` only when a configured layer
  must be temporarily disabled.
- **Breaking:** Automatic global config and sandbox flake scaffolding from `cast run`
  and `cast exec`. Run `cast config init` explicitly before using the shipped
  sandbox harness environment.
- **Breaking:** Hardcoded per-agent environment variable forwarding has been
  completely removed across all harnesses (OpenCode, ClaudeCode, Pi).
  Previously, `cast` automatically passed through common provider API keys
  (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, etc.) from the host. Configuration
  allowlists (`env_passthrough` and `extra_env_passthrough`) are now the sole
  passthrough channel. **Migration:** Add your provider API keys to
  `env_passthrough` in your global `~/.config/cast/cast.json` (e.g.
  `"env_passthrough": ["ANTHROPIC_API_KEY", "OPENAI_API_KEY"]`), otherwise
  agents will lack API credentials inside the sandbox.
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
