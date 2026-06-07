# Project Log

## [bebfde3] Research complete: MCP context and timeout issue

- **Found:** MCP server uses rmcp crate and tokio::process::Command for execution.
- **Found:** Processes dangle because kill_on_drop(true) is missing in crates/cast/src/mcp/exec.rs.
- **Found:** rmcp RequestContext provides a CancellationToken (ct) that triggers on client disconnection.
- **Found:** No global timeout is currently enforced; one can be added to McpConfig and implemented via tokio::time::timeout.

## [bebfde3-dirty] RED phase complete: timeout tests added and failing

161/163 tests pass. 2 fail as expected: test_per_tool_timeout_is_enforced and test_global_timeout_is_enforced. Both fail because execute_tool completes successfully without enforcing any timeout. RED phase is confirmed.

- **Found:** Added global_timeout_secs: Option<u64> to McpConfig in schema.rs.
- **Found:** Added timeout_secs: Option<u64> to McpToolConfig in schema.rs.
- **Found:** Added make_handler_with_timeout and sleep_tool_config helpers in handler.rs tests.
- **Found:** 3 new tests added: test_per_tool_timeout_is_enforced, test_global_timeout_is_enforced, test_execute_tool_future_drop_does_not_hang.
- **Decided:** Sleep duration set to 2s (not 10s) to keep failing tests fast rather than hanging.
- **Decided:** Config deserialization tests are green immediately — they only test schema parsing.

## [f900fc0] GREEN phase complete — timeout and cleanup committed

Commit f900fc0 on feat/mcp-timeout. All 163 tests passing.

- **Found:** kill_on_drop(true) added to tokio::process::Command in exec.rs — processes are now reaped when their future is dropped.
- **Found:** tokio::time::timeout wraps exec::run_command in execute_tool when a limit is configured.
- **Found:** Per-tool timeout_secs takes precedence over global_timeout_secs via Option::or.
- **Found:** Pre-existing cargo fmt issues exist in cli.rs and approval.rs — left unstaged, not part of this commit.
- **Decided:** Only the three directly-modified files were staged to keep the commit atomic.
- **Decided:** Error message format is 'tool execution timed out after {n}s' to satisfy test assertions.

## [733e58c-dirty] Refactor: global_timeout_secs promoted to plain u64

Commit 733e58c on feat/mcp-timeout. 164 tests passing.

- **Decided:** Changed from Option<u64>/None to u64/300 so the field is always serialized and visible in cast config show.
- **Decided:** execute_tool simplified: timeout is now unconditional — removed the if/else between timed and untimed paths.

## [1c18872-dirty] Process group kill implemented and verified

Commit 1c18872 on feat/mcp-timeout. 165 tests passing.

- **Found:** libc and tempfile were already in the workspace — no new dependencies needed.
- **Found:** ProcessGroupGuard RAII pattern is the only safe primitive for async cancellation — CancellationToken/select! is bypassed when a future is dropped from outside.
- **Found:** kill_on_drop(true) retained alongside ProcessGroupGuard: serves a different role (zombie reaping via Tokio background waiter).
- **Found:** test_timeout_kills_entire_process_tree passes: sh writes its PID to a tempfile, sleeps 100s; after 400ms timeout killpg(pgid,0) returns ESRCH confirming entire group is dead.
- **Decided:** Timeout moved into run_command (Duration param + tokio::select!) to co-locate spawn/wait/kill. handler.rs detects timeout by checking err.to_string().contains("timed out").
- **Decided:** cmd.process_group(0) puts child in own PGID; ProcessGroupGuard fires libc::kill(-pgid, SIGKILL) on drop covering all exit paths.
- **Decided:** ESRCH from killpg on already-exited group is silently ignored — idempotent and correct.
- **Open:** Processes that call setsid() (create a new session) escape the process group kill — acceptable for dev tools, would require cgroups to fix.

## [9ccbdf6] refactor: typed ExecError replaces brittle string-match in MCP handler

During review walkthrough of feat/mcp-timeout, identified a brittle string-match in handler.rs (`msg.contains("timed out")`) used to discriminate timeout errors from exec.rs. Consulted Gemini Flash, which recommended changing the return type of run_command to Result&lt;CallToolResult, ExecError&gt; with a manual std::error::Error impl (no thiserror dependency). Applied and committed.

- **Found:** The handler was matching on msg.contains("timed out") — a string literal owned by exec.rs leaked into handler.rs semantics
- **Found:** anyhow::Error implements From<E: std::error::Error> so callers returning anyhow::Result are unaffected by the signature change
- **Found:** color_eyre is compatible with this pattern — both anyhow and color_eyre expose identical downcast APIs and accept any impl std::error::Error
- **Decided:** Change run_command return type to Result<CallToolResult, ExecError> rather than keeping anyhow::Result and using downcast — compile-time safety preferred over runtime discrimination
- **Decided:** Use manual impl std::error::Error rather than adding thiserror — single enum with one variant does not justify the proc-macro dependency
- **Decided:** thiserror to be reconsidered if error surface grows or color_eyre migration proceeds

