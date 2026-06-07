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

