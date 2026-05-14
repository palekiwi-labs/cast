# Trace Report: MCP Tool Call Timeouts and "Ghost Processes"

## 🔍 Incident/Issue Description
During the implementation of the built-in MCP server, research was conducted to understand the behavior of long-running tool calls. It was discovered that the current implementation in `cast` lacks application-level timeouts for tool execution, leading to potential "ghost processes" on the host machine.

## 🕒 Analysis of Current State
- **Mechanism:** `src/commands/mcp/exec.rs` uses `tokio::process::Command` and awaits `child.wait_with_output().await?` indefinitely.
- **Dependency (rmcp):**
    - Standard `tools/call` has no default timeout.
    - MCP "Tasks" feature has a hardcoded **5-minute (300s)** timeout.
- **The Gap:** When a timeout occurs (either at the client level or the 5-minute `rmcp` task limit), the `tokio` future is dropped, but the underlying subprocess continues to run because `kill_on_drop(true)` is not set.

## 📉 Impact
1. **Resource Leakage:** Subprocesses (e.g., heavy builds, infinite loops) can run indefinitely on the host after the agent has disconnected.
2. **Ghost Processes:** The `cast` server loses track of these processes, but they continue to consume CPU and Memory.
3. **Inconsistent State:** The client receives a timeout error, but the operation might still be modifying the filesystem or environment in the background.

## 🛠️ Identified Technical Requirements
To resolve this, the following changes are required in `src/commands/mcp/exec.rs`:
- [ ] Add `kill_on_drop(true)` to the `Command` builder in `run_command`.
- [ ] Implement a `tokio::time::timeout` wrapper around the `wait_with_output` call.
- [ ] Extend `McpToolConfig` (in `src/commands/mcp/config.rs`) to support an optional `timeout_secs` field.
- [ ] Pass the configured timeout down to the execution engine.

## 🏁 Decisions & Rationale
- **Decision:** Use `kill_on_drop(true)` as a mandatory safety net.
- **Rationale:** Prevents orphan processes when the server task is cancelled or times out.
- **Decision:** Introduce tool-level timeout configuration.
- **Rationale:** Different tools (e.g., `read_file` vs `npm_install`) have vastly different expected durations. A global timeout would be too restrictive or too loose.

## 🔗 References
- `rmcp` 1.6.0 Source: `src/task_manager.rs` (`DEFAULT_TASK_TIMEOUT_SECS = 300`)
- `cast` Execution Logic: `src/commands/mcp/exec.rs`
- `cast` Configuration Schema: `src/commands/mcp/config.rs`
