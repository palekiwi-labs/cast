# Project Log

## [03986ba] feat(mcp): resilient server URL resolution via injection [03986ba]

Implemented a stateless MCP URL resolution mechanism that prioritizes environment injection over complex host-side config reading.

Key changes:
- `src/dev/run.rs`: Injects `CAST_MCP_URL` into the agent container at launch, mapping loopbacks to `host.docker.internal`.
- `src/commands/mcp/client.rs`: Implements a simple fallthrough (Flag > Env > Default) for the client.
- Added unit tests for both injection logic and client resolution.
- Dropped `is_running_in_container` and manual `cast.json` parsing in the client to maintain decoupling.

- **Found:** Transitive dependency on `reqwest` via `jsonschema` allows unifying HTTP clients without significant overhead
- **Found:** `host.docker.internal` is a reliable default for local container-to-host communication in `cast`'s target environments
- **Decided:** Use environment injection via `cast run` as the primary discovery mechanism
- **Decided:** Maintain a stateless client that doesn't read host configuration files

## [03986ba] Research complete: rmcp client testing patterns

- **Found:** rmcp uses StreamableHttpClientTransport for HTTP/SSE communication
- **Found:** Testing in rmcp relies on axum mock servers for integration tests
- **Found:** StreamableHttpClient trait allows for unit testing via trait mocking

## [2f452d8-dirty] feat(mcp): implement mcp client and integration tests [2f452d8]

Implemented the MCP client infrastructure using the `rmcp` crate's `StreamableHttpClientTransport`.

Key changes:
- Enabled `rmcp` client features in `Cargo.toml`.
- Created `McpClient` in `src/commands/mcp/client.rs` which handles the SSE-based handshake and tool discovery.
- Implemented `McpClientHandler` to satisfy `rmcp` service requirements.
- Added a robust integration test in `tests/mcp_client_test.rs` using an `axum` mock server to verify the HTTP/SSE lifecycle.

Decisions:
- Used `StreamableHttpClientTransportConfig::with_uri(url).reinit_on_expired_session(true)` to ensure the client automatically recovers from stale sessions.
- Wrapped the `rmcp::Peer` in a stateless `McpClient` struct to provide a clean high-level API for the CLI subcommands.

- **Found:** rmcp::Peer::list_all_tools automatically handles pagination
- **Found:** ClientInfo requires ClientCapabilities and Implementation structs in rmcp 1.6.0
- **Decided:** Use StreamableHttpClientTransport for HTTP/SSE communication
- **Decided:** Enable reinit_on_expired_session for automatic session recovery
- **Decided:** Use axum mock server for integration testing of the HTTP/SSE flow

## [2f452d8-dirty] Research complete: MCP Client Shutdown Hang

- **Found:** RunningService drop detaches the background task without joining it
- **Found:** rmcp HTTP worker has a 5-second timeout for session deletion during cleanup
- **Decided:** Implement explicit McpClient::shutdown to join background tasks
- **Decided:** Call shutdown() in mcp::list_tools to ensure clean CLI exit

## [2f452d8-dirty] Deadlock identified in MCP subcommand integration test

- **Found:** Deadlock occurs when test blocks the executor while the subprocess waits for the test-owned mock server
- **Decided:** Use tokio::task::spawn_blocking for assert_cmd calls in integration tests with mock servers

## [f8f8666-dirty] feat(mcp): implement cast mcp list subcommand [f8f8666]

Completed Slice 2 of the MCP Discovery phase. The cast mcp list subcommand is fully implemented and tested end-to-end.

- **Found:** tokio::task::spawn_blocking is required when using assert_cmd inside a tokio::test that also hosts a mock server — omitting it starves the executor and deadlocks the test
- **Found:** rmcp RunningService::cancel() must be called explicitly; dropping the service detaches the background task and leaves a 5-second session-deletion cleanup running in the runtime
- **Decided:** Expose McpClient::shutdown() to wrap RunningService::cancel() for clean CLI exit
- **Decided:** Split McpCommands::List dispatch from verify_config in cli.rs so listing tools does not require an approved config
- **Decided:** Use spawn_blocking for all assert_cmd subprocess calls in tokio integration tests

## [bfa8356] Pivot to Minimalist JSON-only MCP Client Strategy

- **Found:** Custom CLI argument parsing adds significant complexity and potential for 'Type Gap' errors in a dynamic MCP environment
- **Found:** Stdin support is sufficient for scripting and piping needs in a dev tool like cast
- **Decided:** Use pure JSON for MCP tool arguments (no custom flag parsing/coercion)
- **Decided:** Support only Inline JSON strings and Stdin for tool calls (skip @file convention)
- **Decided:** Skip local client-side validation via jsonschema; rely on server-side validation instead
- **Decided:** Implement 'describe' subcommand to display tool schemas to the user

## [fa01c11] feat(mcp): implement cast mcp describe subcommand [fa01c11]

Implemented Slice 3 of the MCP client. The `cast mcp describe <tool>` subcommand fetches the tool list from the server and pretty-prints the tool's name, description, input schema properties (with type, required/optional, description), and an example invocation hint.

- **Found:** Describe dispatch can share the config-free fast path with List — no verify_config needed
- **Found:** The mock tool needed a real inputSchema (with properties/required) to meaningfully test the schema renderer; updated make_mock_tool() helper shared by all tests
- **Decided:** Placed print_tool_schema as a pub fn in mod.rs for direct unit-testability without a running server
- **Decided:** Unknown tool name produces anyhow error pointing user to cast mcp list
- **Decided:** describe_tool dispatched from cli.rs alongside List, outside the verify_config gate

## [c396171] feat(mcp): implement cast mcp call subcommand [c396171]

Completed Slice 4. The cast mcp call subcommand is fully implemented and tested end-to-end, including a fix for Nix sandbox build failures.

- **Found:** rustls-platform-verifier panics during reqwest::Client::builder().build() in Nix sandboxes where /etc/ssl/certs is absent
- **Found:** std::env::set_var is thread-unsafe in parallel test suites — the correct approach is to inject SSL_CERT_FILE at the Nix derivation level
- **Found:** pkgs.cacert in nativeBuildInputs + SSL_CERT_FILE derivation env var is the canonical Nix fix for rustls certificate loading in sandboxed builds
- **Found:** McpCommands::Call must be dispatched directly in cli.rs (bypassing verify_config) since it is stateless and does not use cast.json
- **Decided:** Use pkgs.cacert in flake.nix rather than dummy cert files or std::env::set_var in tests
- **Decided:** Dispatch McpCommands::Call directly alongside List and Describe, not through the verify_config gate

