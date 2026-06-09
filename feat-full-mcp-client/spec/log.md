# Project Log

## [5916dcd] S1 complete: config module committed

Implemented and committed the config module foundation for cast-mcp-client. All 7 TDD cycles executed: parse, defaults, env-var substitution, merge, file loading, missing-file skip, malformed-config fallback.

- **Found:** All 9 config unit tests + 2 existing lib unit tests + 9 integration tests pass (20 total)
- **Found:** Builder did not commit — had to commit manually after verification
- **Found:** No new dependencies needed — serde/serde_json already present
- **Decided:** Commit: feat(mcp-client): add config module with loading and env substitution (5916dcd)
- **Decided:** Only lib.rs change: pub mod config; added at top

## [48c3b51] S2 complete: server map logic + env var rename committed

All three parts of Slice 2 implemented and committed (48c3b51). TDD cycles executed for both new functions. Parallel env-var test race fixed with a static Mutex.

- **Found:** test_resolve_cast_mcp_url_env_wins_over_config was flaky due to parallel tests racing on CAST_MCP_CLIENT_URL — fixed with static Mutex<()> ENV_LOCK held for the duration of each env-sensitive test
- **Found:** build_server_map correctly distinguishes between 'URL from flag/env' (bare cast entry, no headers) and 'URL from config' (full entry preserved) via the Option<String> argument
- **Found:** opencode.json also needed updating to CAST_MCP_CLIENT_URL to keep the dev environment working
- **Decided:** Use static Mutex<()> (not serial crate) to serialise env-var tests — zero extra dependencies
- **Decided:** cast_url: Option<String> to build_server_map encodes the flag/env vs config distinction at the type level — no extra boolean needed
- **Decided:** Commit all three parts (resolve, build_server_map, env rename) in a single atomic commit

## [5515d0d] Refactor: resolve_cast_mcp_url made pure, Mutex removed

- **Found:** cast-mcp-client does not use figment at all — config is hand-rolled serde_json, so env vars are not automatically merged into the config struct
- **Found:** The correct fix for env-var test races is not a Mutex but eliminating the env read from inside the function entirely
- **Found:** Consulted Gemini Flash — unanimous recommendation for Option B (inject env_url as parameter)
- **Decided:** resolve_cast_mcp_url takes explicit: Option<String>, env_url: Option<String>, config: &ClientConfig — pure function, no side effects
- **Decided:** Caller (main.rs, S4) does std::env::var("CAST_MCP_CLIENT_URL").ok() at the binary boundary and passes it in
- **Decided:** static Mutex and all unsafe env mutation deleted from tests

## [5763b19] Revert: CAST_MCP_URL env var name restored

- **Found:** cast figment loader uses Env::prefixed("CAST_").split("__") but McpConfig has no url field — CAST_MCP_URL is not swept into any meaningful config field, so no collision exists
- **Found:** cast-mcp-client does not use figment at all, so the CAST_MCP_CLIENT_ prefix was solving a non-existent problem
- **Decided:** Keep CAST_MCP_URL as the env var name injected by cast run into the container
- **Decided:** resolve_cast_mcp_url in S4 main.rs will read std::env::var("CAST_MCP_URL").ok()

## [5763b19-dirty] S3 complete: McpClient::connect accepts RemoteServerConfig with custom headers

Implemented and verified S3. McpClient::connect now takes &RemoteServerConfig instead of &str. Custom headers (HashMap<String, String>) are converted to HashMap<HeaderName, HeaderValue> and forwarded via StreamableHttpClientTransportConfig::custom_headers(). New integration test uses axum middleware to capture the header value server-side and assert it was received.

- **Found:** http crate is NOT a transitive dep visible to cast-mcp-client — had to add http = '1' explicitly to Cargo.toml
- **Found:** HeaderName::from_str requires the std::str::FromStr trait in scope — it is a trait impl, not an inherent method
- **Found:** axum middleware::from_fn captures headers correctly on the first HTTP request (initialize), which is sufficient for the assertion
- **Found:** All 30 tests (20 unit + 10 integration) pass cleanly after the change
- **Decided:** McpClient::connect(server: &RemoteServerConfig) — new canonical signature
- **Decided:** Command helper functions (list_tools_cmd, describe_tool_cmd, call_tool_cmd) build a bare RemoteServerConfig { url, headers: HashMap::new(), enabled: true } internally — S4 will replace this with full config-driven server maps
- **Decided:** std::collections::HashMap promoted to top-level use in lib.rs — removes all inline std::collections::HashMap paths
- **Decided:** http = '1' added as direct dependency in Cargo.toml

## [eafa5bc] S4 complete: CLI wiring + --cast-mcp-url rename committed

All four TDD cycles executed. config::load() wired into main.rs, --url renamed to --cast-mcp-url, --server flag added to list, command function signatures updated to accept server maps, new integration test for config-driven URL resolution.

- **Found:** CAST_MCP_URL is injected by cast into the agent container — the new config-fallback test had to call .env_remove("CAST_MCP_URL") on the subprocess command to prevent the ambient env var from winning over the project-local config
- **Found:** tempfile = "3" added to dev-dependencies for test temp dir creation
- **Found:** pick_server() helper introduced in lib.rs: prefers 'cast' entry if present, otherwise takes first entry — bridges old single-server behaviour with new server-map API until S5 rewrites list properly
- **Found:** resolve_server_url() (old single-server helper) is now dead code but retained since it still has unit tests; can be removed in a future cleanup
- **Decided:** list_tools_cmd, describe_tool_cmd, call_tool_cmd all accept HashMap<String, RemoteServerConfig> instead of Option<String>
- **Decided:** main.rs reads CAST_MCP_URL at the binary boundary and passes it as env_url to resolve_cast_mcp_url — pure function, no env reads inside library code
- **Decided:** --server flag added to List variant in clap; passed as _server_filter to list_tools_cmd (ignored for now — S5 implements the actual filtering)
- **Decided:** Commit: feat(mcp-client): S4 — wire config load, rename --url to --cast-mcp-url, add --server flag (eafa5bc)

## [6863ae4] S5 complete: multi-server prefixed list committed

- **Found:** The S5 test block was accidentally inserted before the spawn_mock_server helper and the call/describe tests, causing those tests to disappear from the file — had to restore them from git show HEAD
- **Found:** axum Request/Next/Mutex imports were at the top level but only needed in test_headers_are_sent_to_server; moved them into that function's scope to fix unused-import warnings
- **Found:** futures::future::join_all used for concurrent server queries; futures = '0.3' added as a dependency
- **Decided:** list_tools_cmd validates the --server filter name against the server map before making any connections — fails fast with a clear error if unknown
- **Decided:** Empty server map produces bare '[]' output (no connections attempted)
- **Decided:** Tool name prefix applied as '{server_name}/{tool_name}' inside list_tools_cmd after fetching; the Tool.name field is mutated in-place before collecting
- **Decided:** Commit: feat(mcp-client): S5 — list with multi-server concurrent fetch and server/tool name prefix (6863ae4)

## [a35777e] [a35777e] S6 complete: describe/call require server/tool format with routing

- **Found:** pick_server() used a heuristic (prefer 'cast', else first entry) — replaced entirely by explicit server/tool parsing
- **Found:** test_routing_unknown_server_fails was already passing in RED because pick_server errored on an empty map when a ghost server was passed; the other three tests were correctly RED
- **Found:** The --cast-mcp-url flag injects the server under key 'cast' in the map, so all existing tests updated cleanly to cast/tool_name format with no other changes
- **Found:** describe output keeps the bare tool name (no server/ prefix) — only list adds the prefix
- **Decided:** parse_server_tool(&str) -> Result<(&str, &str)> is a private pure helper using split_once('/') — minimal and zero-allocation
- **Decided:** describe_tool_cmd and call_tool_cmd both parse the server/tool ref first, look up the server map, then connect only to the target server
- **Decided:** pick_server() deleted — dead code once routing is explicit
- **Decided:** 6 existing integration tests updated: bare tool names replaced with cast/tool_name to match the new required format

## [ed007cb] [ed007cb] S7 complete: list skips unreachable servers with named warning

- **Found:** list_tools_cmd already had per-server error handling (eprintln Warning) but the warning message did not include the server name — only the raw error string
- **Found:** The fix required wrapping the inner async block to return (server_name, Result<Vec<Tool>>) instead of Result<Vec<Tool>> so the server name is available at the error-handling site
- **Found:** All 40 tests pass (20 unit + 20 integration)
- **Decided:** Each future in join_all now resolves to (String, Result<Vec<Tool>>) — the server name is threaded through the result tuple
- **Decided:** Warning format: 'Warning: server \'{}\' is unreachable: {}' — includes server name first for easy grep/identification
- **Decided:** Exit code remains 0 when at least one server fails — only an unknown --server filter causes non-zero exit

## [0d1fc46-dirty] [0d1fc46] S8 complete: status command with concurrent health checks

- **Found:** status_cmd follows the same (name, url, Result) tuple pattern as the S7 list fix — natural fit since errors need to be attributed per server
- **Found:** Sort by server name after join_all ensures deterministic JSON output regardless of which futures resolve first
- **Found:** All 41 tests pass (20 unit + 21 integration)
- **Decided:** Output schema: { name, url, status: ok|error, error? } — error field present only on failure entries
- **Decided:** status_cmd always exits 0; per-server failures are inline JSON, not stderr warnings (different from list which warns on stderr)
- **Decided:** Status subcommand accepts --cast-mcp-url for parity with other commands

## [3921a90] Cleanup: remove dead resolve_server_url

Removed resolve_server_url (the old single-server helper with a hardcoded default URL) and its two unit tests. This also resolves the todo artifact about not defaulting to a cast MCP URL when none is provided.

- **Found:** resolve_server_url was truly dead — no callers outside lib.rs itself
- **Found:** Removing it also eliminated the last unsafe env::set_var usage in the test suite
- **Found:** 39 tests remain after deletion (18 unit + 21 integration), all green
- **Decided:** Delete resolve_server_url and both its tests in a single commit (3921a90)
- **Decided:** Mark todo/no-default-for-cast-mcp-url.md as done

