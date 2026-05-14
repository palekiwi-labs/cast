# Project Log

## [ffc6b29] Research complete: Built-in MCP Server

- **Found:** Semantic Tools with dynamic JSON schemas
- **Found:** Manual ServerHandler implementation for runtime tool registration

## [ffc6b29] Saved Implementation Plan

- **Decided:** Documented phases and created todo roadmap

## [ffc6b29] Defined Todo Slices

- **Decided:** Structured todo roadmap into TDD/git-commit compliant slices

## [fe833a8] Slice 1: Configuration Schema & Dependencies

- **Found:** Successful deserialization of heterogeneous ArgTemplate array
- **Decided:** Gated mcp dependencies behind a default feature flag

## [5925541-dirty] Code Review: MCP Configuration & Mapper

- **Found:** Logical overlap in conditional argument evaluation
- **Found:** Strong security against shell injection by design
- **Decided:** Will update conditional evaluation to use logical AND in next slice

## [70cf457] Fixed conditional logic in parameter mapper

- **Found:** Logical AND correctly applied for multiple conditions
- **Decided:** Kept code review file in root per user instruction

## [d8ef250] Slice 3: Secure Subprocess Execution Sandbox

- **Found:** Nix-hermetic testing pattern with logic decoupling
- **Found:** Graceful execution error reporting for MCP tools
- **Decided:** Added TMPDIR retention for Nix sandbox compatibility
- **Decided:** Added optional working_dir for tool isolation
- **Decided:** Enabled tokio 'process' feature

## [9d41c89] Slice 4: dynamic MCP handler implemented

- **Found:** CallToolResult is #[non_exhaustive] in rmcp 1.6.0 — must use ::success() constructor then mutate is_error; request.arguments is Option<JsonObject> not Option<Arc<JsonObject>>; ListToolsResult has an undocumented meta field requiring Default::default(); str::as_str() is unstable on Cow<str>, use &* deref instead
- **Decided:** Bypassed #[tool_router] macro and implemented ServerHandler manually for runtime tool loading; transport-level integration tests deferred to Slice 5 — unit tests cover all business logic without needing RequestContext; extracted tool_config_to_rmcp_tool as a public pure function to keep handler tests simple and deterministic

## [b0a41a3] Pre-compile jsonschema validators

- **Found:** jsonschema::Validator is the correct type to store
- **Decided:** Compile all tool schemas during McpHandler initialization to save per-request overhead

## [3bc7c8e] Correct error code for missing tools

- **Found:** MCP spec suggests InvalidParams is better for invalid parameters to a valid method
- **Decided:** Change MethodNotFound to InvalidParams in call_tool when the tool name is unknown

## [748a69b] Add tracing to MCP handler

- **Found:** Errors in tool execution were silent in logs
- **Decided:** Add tracing::info/warn/error calls to provide observability into tool execution flow

## [7448623] Fix: InvalidParams for unknown tool name

- **Found:** rmcp 1.6.0 exposes ErrorData::invalid_params(message, data) directly — no special wrapper needed
- **Decided:** Use McpError::invalid_params instead of method_not_found when tool lookup fails; method_not_found is semantically wrong because call_tool (the RPC method) does exist

## [0874e5e] feat: tracing instrumentation in call_tool

- **Found:** tracing crate already in dependency tree via the mcp feature; no new dep needed
- **Decided:** Use structured field syntax (tool = %name) so log aggregators can index by tool name without parsing message strings

## [6272250] refactor: extract execute_tool for testability

- **Found:** rmcp RequestContext cannot be trivially constructed in tests (requires MPSC Peer wiring); this extraction unblocks pipeline test coverage
- **Decided:** Keep execute_tool pub(crate) rather than pub to avoid leaking internals across crate boundary; call_tool becomes a thin delegation shim

## [80cab23] perf: pre-compile jsonschema validators at startup

- **Found:** jsonschema::Validator derives Clone so it fits naturally in Arc<HashMap> without boxing
- **Decided:** Store validators as Result<Validator, String> in Arc<HashMap> — Err entries surface schema problems at call time with a clear error message rather than panicking at startup, which preserves service availability when only one tool has a bad schema

## [2ac9e9d] test: pipeline integration tests for execute_tool

- **Found:** CallToolRequestParams::new(name).with_arguments(map) is the clean constructor path; JsonObject is serde_json::Map<String, Value> so json!({}).as_object().unwrap().clone() is the idiomatic way to build it in tests
- **Decided:** Assert error codes numerically (-32602, -32600) rather than matching error message strings — codes are stable per spec; messages are not

## [5c24904] test: fix nix build via PATH inheritance

- **Found:** The nix build failed because 'echo' could not be found with an empty PATH in the sandbox, causing a spawn error which was correctly flagged by our engine.
- **Decided:** Pass through PATH from the test runner to the McpHandler in integration tests. This allows subprocesses like 'echo' to be found in the Nix sandbox without compromising the 'Default-Deny' security posture of the production execution engine.

## [4051787] Refactored McpHandler for compliance and performance

- **Found:** Schema violations returned -32600 instead of -32602; initialization swallowed errors.
- **Decided:** Adopted Inner Pattern, fail-fast Result initialization, and pre-computed tool list.
- **Open:** Integrate McpHandler into the actual server startup in Slice 5.

## [f8f17b1] Slice 5: Server infrastructure implemented

- **Found:** tokio signal and net features were missing from Cargo.toml; SSE keep-alives already on by default (15s) in rmcp 1.6.0.
- **Decided:** Used axum with_graceful_shutdown over CancellationToken (single server, no background workers); ApprovedConfig gate enforced at CLI boundary before runtime creation.

## [bd1a66e] Security: Restricted MCP bind address

- **Found:** Binding to 0.0.0.0 by default exposed the host to the local network.
- **Decided:** Moved default to 127.0.0.1; added --host flag for explicit opt-in.

## [ac49d73] Apply effective defaults to MCP configuration

- **Found:** MCP config display showed null for unset port/hostname, which was misleading since CLI defaults were applied later.
- **Decided:** Moved defaults (8080/127.0.0.1) into the McpConfig schema using serde defaults and synchronized CLI overrides to respect the config file.

## [6063f41] Added terminal logging for MCP server

- **Found:** Users had no visibility into MCP server activity on the host terminal.
- **Decided:** Added eprintln! logging to McpHandler to show discovery, tool calls, mapped commands, and exit status.

## [b7c9b94] Research complete: MCP Documentation Tools

- **Found:** Identified McpHandler in src/commands/mcp/handler.rs as the location for adding built-in documentation tools.
- **Found:** Verified rmcp support for Resources, though Tools remain the preferred interface for now.
- **Found:** Proposed a registry-based approach for serving documentation files from the repository.

## [b7c9b94] Research complete: MCP Session not found

- **Found:** The error is a 404 from rmcp when a session ID is unknown.
- **Found:** Sessions are in-memory and subject to a 5-minute inactivity timeout.
- **Found:** The server lack persistence, so restarts wipe all sessions.

## [6060736] Research complete: MCP Tool Call Timeouts

- **Found:** Subprocesses in exec.rs lack timeouts and kill_on_drop(true), leading to ghost processes
- **Found:** rmcp enforces a 5-minute timeout on the Tasks feature but not on standard tool calls
- **Decided:** Implement tool-level timeout configuration in cast.json
- **Decided:** Enable kill_on_drop(true) for all MCP tool subprocesses

