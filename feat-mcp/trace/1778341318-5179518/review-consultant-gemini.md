# Code Review: MCP Subprocess Execution Sandbox (Slice 3)
**Reviewer:** `@consultant-gemini`
**Status:** Completed
**Commit:** `5179518`

## Summary
The "Logic Decoupling" pattern is applied effectively. Passing the environment as a pure `HashMap` instead of relying on implicit `std::env` state perfectly addresses the thread safety and testability constraints.

## Feedback & Recommendations

### 1. Robustness: Graceful Error Handling for `spawn()`
**Observation:** Currently, `cmd.spawn()?` and `child.wait_with_output()?` propagate `std::io::Error` via `anyhow::Result`, which may cause the MCP server to return a JSON-RPC internal error.
**Recommendation:** Catch execution-level failures (e.g., binary not found) and return them as an MCP tool error (`is_error: true`). This allows the LLM to understand and react to the failure reason.

### 2. Nix Compatibility: Retaining `TMPDIR`
**Observation:** `cmd.env_clear()` wipes all variables. In Nix sandboxes, access to `/tmp` is restricted, and tools rely on `TMPDIR`.
**Recommendation:** Retain `TMPDIR` alongside `PATH` in `resolve_env` to ensure tools can safely write to temporary storage during Nix builds/tests.

### 3. Idiomatic Rust: Unnecessary Cloning
**Observation:** `run_command` performs an unnecessary deep clone of `McpEnvConfig`.
**Recommendation:** Use references to avoid allocation:
```rust
let default_env = McpEnvConfig::default();
let env_config = tool.env.as_ref().unwrap_or(&default_env);
let resolved_env = resolve_env(env_config, host_env);
```

### 4. Security: Working Directory Isolation
**Observation:** Subprocesses inherit the current working directory of the host process.
**Recommendation:** Consider enforcing an explicit working directory via `cmd.current_dir(...)` to further tighten the sandbox.

### 5. Stdout/Stderr Interleaving
**Observation:** Sequential combination (`stdout` then `stderr`) loses temporal ordering.
**Recommendation:** Acceptable for now, but consider OS-level redirection if precise interleaving becomes necessary for specific tools.
