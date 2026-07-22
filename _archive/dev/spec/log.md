# Project Log

## [e230c3a] Research complete: Custom flake location configuration

- **Found:** 'use_flake_path' configuration field already exists in the schema.
- **Found:** The logic in 'build_command.rs' correctly handles the custom path.
- **Found:** The feature is currently undocumented.

## [e230c3a] Research complete: AI agent commit signing context

- **Found:** Agents currently inherit repository-local git identity due to workspace bind-mounting.
- **Found:** GPG signing is explicitly disabled in agent base images.
- **Found:** No mechanism exists to provide separate signing keys to agents without exposing host keys.
- **Found:** Environment passthrough is limited to a whitelist that excludes git identity variables.

## [abe4ba0] Research complete: Timezone configuration in dev containers

- **Found:** 'cast run' automatically mounts '/etc/localtime:/etc/localtime:ro' from the host.
- **Found:** Users can override or set the 'TZ' environment variable via 'cast.env' files (global or local).
- **Found:** CLI arguments to 'cast run' are passed to the agent, not to 'docker run', so '-e' flags cannot be used in the CLI.
- **Found:** Verification can be done with 'cast run o -- date'.

## [73dff8d] Research complete: cast-mcp-client generalization

- **Found:** cast-mcp-client is currently hardcoded to HTTP/SSE via rmcp.
- **Found:** rmcp supports stdio (TokioChildProcess) and full OAuth 2.0 with custom headers.
- **Found:** opencode uses a mature 'mcp' config format that can be adapted for cast-mcp-client.json.
- **Found:** Implementation requires configuration loading, transport abstraction, and CLI updates.

## [1506bfd] Research `nix develop` integration for `cast shell`

Researched how `nix develop` is integrated into `cast`.
Found that `build_command.rs` handles the wrapping logic for `cast run`.
`shell.rs` handles `cast shell` but currently hardcodes `/bin/bash`.
Global flake is at `~/.config/cast/nix/` and is mounted into the container.
Plan to implement the feature by adding a `--dev` flag to `cast shell` and using `Agent::build_command` to wrap a bash shell.

- **Found:** `build_command.rs` implements nested `nix develop` wrapping.
- **Found:** `shell.rs` currently uses a simple `docker exec` with `/bin/bash`.
- **Found:** Flake detection logic is in `run.rs` and should probably be shared.
- **Decided:** Add `--dev` flag to `ShellAgent` variants in `cli.rs`.
- **Decided:** Update `dev::shell` to support starting in a devshell using `build_command` logic.

## [1506bfd] Research complete: Allow starting shell in devshell

- **Found:** cast shell is currently a direct docker exec into /bin/bash
- **Found:** nix develop wrapping logic exists in build_command.rs and is used by cast run
- **Found:** global flake is already detected and mounted at ~/.config/cast/nix/
- **Found:** ShellAgent enum in cli.rs needs to be updated to support flags

