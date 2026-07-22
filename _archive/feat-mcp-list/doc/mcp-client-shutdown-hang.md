# Research: MCP Client Shutdown Hang

## Problem
The `cast mcp list` subcommand hangs for several seconds (up to 5s) before exiting, causing integration tests to timeout or appear stuck.

## Root Cause: Detached Background Tasks
Research into `rmcp` v1.6.0 reveals that the `RunningService` handle for the client performs asynchronous cleanup when dropped.

1. **`Drop` vs `Close`**: `RunningService` implements `Drop` by triggering a `CancellationToken`, but it does not await the background `JoinHandle`. The task is detached.
2. **Session Deletion**: The `StreamableHttpClientWorker` (background task) attempts to call `delete_session` on the server after the main loop breaks. This call has a **5-second timeout**.
3. **Runtime Blocking**: The Tokio runtime in `cast` waits for all spawned tasks to finish. It remains blocked by the detached session-deletion task until it completes or hits the 5s timeout.

## Evidence
- `src/service.rs`: `RunningService::drop` only cancels the token.
- `src/transport/streamable_http_client.rs`: `StreamableHttpClientWorker::run` performs a timed-out `delete_session` call during its cleanup phase.
- `src/transport/worker.rs`: `WorkerTransport::close` explicitly joins the worker handle, proving that `close()` is the intended path for synchronous cleanup.

## Recommendation
- Expose a `shutdown()` method on `McpClient` that calls `_service.close().await`.
- Update the CLI subcommand to explicitly call `shutdown()` after printing the tool list.
