# Trace: MCP Subcommand Test Deadlock

## Context
We are implementing the `cast mcp list` subcommand. To verify its behavior, we added an integration test in `tests/mcp_client_test.rs` that:
1. Spawns an `axum` mock MCP server on the test's Tokio runtime.
2. Invokes the `cast` binary as a subprocess using `assert_cmd`.
3. Asserts on the subprocess output.

## Problem: Deadlock during Shutdown
The test `test_mcp_list_subcommand_output` hangs indefinitely.

### Analysis
The hang is a classic deadlock between the test process and its child process:
1. **Subprocess (cast)**: Executes `list_tools`, prints the output, and then calls `mcp_client.shutdown().await`.
2. **Handshake/Cleanup**: `shutdown()` triggers a `delete_session` request (HTTP DELETE) to the mock server to clean up the SSE session.
3. **Blocking Assert**: In the test process, `Command::assert()` is a **blocking** call. It halts the thread execution until the subprocess exits.
4. **Executor Starvation**: Because the test process is blocked waiting for the subprocess, the Tokio executor (especially if using `current_thread` flavor) cannot run the task responsible for the `axum` mock server.
5. **Wait Loop**: The subprocess is waiting for a response to `delete_session` from the mock server. The mock server is waiting for the test process to yield the thread so it can process the request. The test process is waiting for the subprocess to exit.

### Evidence
- The hang occurs specifically after the subprocess should have finished its work.
- `rmcp` v1.6.0 includes a 5-second timeout for `delete_session`, but if the network stack is blocked or the executor is starved, it may manifest as an indefinite hang or a very long delay depending on how the runtime handles the blocking call.
- The use of `std::process::Command` (via `assert_cmd`) inside a `tokio::test` without `spawn_blocking` is a known anti-pattern when the test process must also act as a server for the subprocess.

## Proposed Resolution
Wrap the blocking `assert_cmd` call in `tokio::task::spawn_blocking` to ensure the Tokio reactor remains active and can handle the mock server's requests while the test waits for the subprocess to complete.
