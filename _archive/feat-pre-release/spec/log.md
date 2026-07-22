# Project Log

## [d34c88f] Add MIT License

Added MIT license file to root and updated Cargo.toml files for both crates to include license metadata.

- **Decided:** Use MIT license as it matches the project's dependency ecosystem.

## [fcf8db6] Add CHANGELOG.md

Created CHANGELOG.md following 'Keep a Changelog' format with the initial 0.1.0 release entry.

## [02fa449] Add Taskfile.yml

Added Taskfile.yml with a 'prepare-release' task to automate tagging and pushing releases from the master branch.

## [0b9969c] Update root README.md

Updated root README.md with project description, Nix installation instructions, and links to documentation.

## [9cbc2b0] Add project-level docs

Created project-level documentation in the 'docs/' directory, including a README and a Quick Start guide focused on the Nix installation and usage path.

## [1ec497f] Add cast crate docs

Added comprehensive documentation for the 'cast' crate, covering getting started, concepts, command reference, agents, configuration, Nix integration, and MCP server features.

## [bc7830a] Add cast-mcp-client crate docs

Added documentation for the 'cast-mcp-client' crate, including usage, configuration, and script generation guides.

## [831c067] Reformat docs to 80 chars

Reformatted all documentation files (including project-level, crate-level, and root files) to adhere to an 80-character line length limit for better readability and standard compliance.

## [715ed91] Fix long lines in docs

Converted absolute GitHub URLs to relative repository paths in concepts.md to strictly adhere to the 80-character line length limit. All documentation files are now within the 80-char limit.

## [219eddb] Document global flake location

Confirmed in source code (src/dev/run.rs) that cast looks for a global flake at ~/.config/cast/nix/flake.nix and updated the documentation in crates/cast/docs/nix/flake-integration.md accordingly.

- **Found:** The global flake is checked at `~/.config/cast/nix/flake.nix` in `crates/cast/src/dev/run.rs`.

## [aa60583-dirty] Update MCP client examples

Updated MCP client usage examples in `crates/cast-mcp-client/docs/usage.md` and `crates/cast/docs/mcp/client.md` to use the built-in `cast` server name and actual built-in tools (`list_cast_documentation`, `fetch_cast_documentation`) instead of placeholder values. Also fixed the command syntax in these docs (adding the required `<server>` argument).

## [20a8f44] Document extra data volumes

Added documentation for `volumes_namespace` and `extra_data_volumes` in `config/reference.md`, `concepts.md`, and `config/env-overrides.md`. Also reformatted several files to strictly adhere to the 80-character line length limit.

- **Found:** `extra_data_volumes` allows configuring Docker named volumes and bind mounts.
- **Found:** `volumes_namespace` defaults to `"cast"`.

