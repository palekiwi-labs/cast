# Plan: cast-mcp-client General HTTP MCP Client

## Goal

Evolve `cast-mcp-client` from a single-server cast-only HTTP client into a
general-purpose client for any HTTP-based MCP server, driven by a
`cast-mcp-client.json` config file.

## Out of Scope (Future Phase)

- stdio / local process transport
- OAuth (any form — browser or client credentials)

---

## Design Decisions

| Decision | Choice |
|---|---|
| Transport | HTTP remote only (no stdio) |
| Auth | Static headers with `{env:VAR}` substitution (covers Bearer tokens, API keys) |
| `list` output | Grouped by server: `{ "name": { "status", "tools" / "error" } }` |
| Tool reference format | `server/tool` — always required, no bare-name shorthand |
| `--url` rename | `--cast-mcp-url` (makes cast-specific purpose explicit) |
| Config locations | `~/.config/cast/cast-mcp-client.json` (global) + `./cast-mcp-client.json` (project, wins on conflict) |
| Multi-server resolution | All `enabled: true` config entries + optional `"cast"` entry from flag/env |
| Bad server behaviour | Listed as `"unreachable"` in output, never blocks other servers |
| Malformed config | Warning to stderr, silently skipped, falls back to `Default` |

---

## Configuration Schema

```jsonc
// cast-mcp-client.json
{
  "mcp": {
    "cast": {
      "url": "http://127.0.0.1:8080/mcp"
      // no auth — internal cast server
      // also populated by CAST_MCP_URL env var or --cast-mcp-url flag
    },
    "sentry": {
      "url": "https://mcp.sentry.dev/mcp",
      "headers": {
        "Authorization": "Bearer {env:SENTRY_TOKEN}"
      }
    },
    "context7": {
      "url": "https://mcp.context7.com/mcp",
      "headers": {
        "CONTEXT7_API_KEY": "{env:CONTEXT7_API_KEY}"
      },
      "enabled": false   // opt-out without removing the entry
    }
  }
}
```

Fields per server entry:
- `url` (required): HTTP endpoint
- `headers` (optional): key-value map; values support `{env:VAR}` substitution
- `enabled` (optional, default `true`): set to `false` to skip without deleting

---

## Cast Server Resolution Priority

The `"cast"` server entry is special — it can be sourced three ways (first wins):

1. `--cast-mcp-url` CLI flag
2. `CAST_MCP_URL` environment variable
3. `mcp.cast.url` in config file

If none of the above is set → no `"cast"` entry in the server map → cast MCP is
simply absent from results. Not an error.

When path 1 or 2 wins, it overrides any `"cast"` entry in the config
(including its headers). When path 3 is used, the full config entry is used
as-is (including any headers defined there).

---

## Key Implementation Notes

### McpClient::connect signature change

The existing `connect(url: &str)` becomes `connect(server: &RemoteServerConfig)`.
Headers are converted from `HashMap<String, String>` to `http::HeaderName`/`http::HeaderValue`
at connection time (no shortcut exists in rmcp — manual conversion required via `HeaderName::from_str`).

### Custom headers in tests (header verification)

rmcp's `ServerHandler` trait operates at the MCP application layer and does not expose
raw HTTP headers. To verify headers in tests, an axum middleware layer must intercept
requests before routing to the MCP service. Pattern:

```rust
let router = axum::Router::new()
    .nest_service("/mcp", mcp_service)
    .layer(axum::middleware::from_fn(|req, next| async move {
        // inspect req.headers() here, store in shared Arc<Mutex<...>>
        next.run(req).await
    }));
```

### Config injection in CLI integration tests

`config::load()` reads `~/.config/cast/cast-mcp-client.json` (global) and
`./cast-mcp-client.json` (project-local). Integration tests that need a custom
config write a temp `cast-mcp-client.json` and set the command's working directory
to that temp dir via `cmd.current_dir(tmpdir)`. The global config file is absent in
CI, so only the project-local file is used.

### Concurrency in list

`list_tools_cmd` contacts all servers concurrently using `futures::future::join_all`.
Each future resolves to `(name, Result<Vec<Tool>, _>)`. Errors are mapped to
`"unreachable"` status; they never propagate out or block other servers.
`futures` is a transitive dependency via rmcp — no new dep needed.

---

## Breaking Changes

- `list` output changes from a flat `[{...}]` array to `{ "server": { ... } }`.
  No known downstream consumers of this CLI output (opencode uses `CAST_MCP_URL`
  directly, not this CLI).
- `describe` and `call` now require `server/tool` format instead of bare `tool`.
  Existing callers (if any) must be updated.
- `--url` flag is removed; replaced by `--cast-mcp-url`.

---

## Implementation Slices

Work is divided into seven vertical TDD slices. Each slice is independently
committable: tests are written first (RED), then implementation (GREEN).

### S1 — Config module

**Adds:** `src/config.rs` (new file), `pub mod config` in `src/lib.rs`.

**Behaviors tested (unit tests in `config.rs`):**
- Parses a minimal JSON config into `ClientConfig` with correct field values
- `enabled` defaults to `true` when omitted from JSON
- `headers` defaults to empty map when omitted from JSON
- Server with `"enabled": false` is present in the raw parsed map (filtering
  is `build_server_map`'s job, not `load()`'s)
- `{env:VAR}` in a header value is replaced with the env var at load time;
  unset vars are replaced with empty string (or left as-is — decide at impl)
- Project-local config entries replace global entries of the same server name
  (full replacement, no deep field merge)
- Missing config files are silently skipped

**Public API introduced:**
```rust
pub struct ClientConfig {
    pub mcp: HashMap<String, RemoteServerConfig>,
}
pub struct RemoteServerConfig {
    pub url: String,
    pub headers: HashMap<String, String>,
    pub enabled: bool,
}
pub fn load() -> ClientConfig
pub fn load_from_files(
    global: Option<&std::path::Path>,
    project: Option<&std::path::Path>,
) -> ClientConfig  // exposed for tests
```

**Files:** `src/config.rs` (new), `src/lib.rs` (add `pub mod config`)

---

### S2 — Server map logic

**Adds:** `resolve_cast_mcp_url` and `build_server_map` as pub functions in `src/lib.rs`.

**Behaviors tested (unit tests in `lib.rs`):**
- `resolve_cast_mcp_url`: explicit flag wins over env var wins over config entry
- `resolve_cast_mcp_url`: returns `None` when no source provides a URL
- `build_server_map`: includes all `enabled: true` servers from config
- `build_server_map`: excludes servers with `enabled: false`
- `build_server_map`: injects a bare-URL `"cast"` entry (no headers) when
  `cast_url` is `Some(url)` and that URL came from flag/env (i.e., overrides
  any headers a config `"cast"` entry may have had)
- `build_server_map`: uses the config's full `"cast"` entry (including headers)
  when the URL was sourced from the config itself

**Public API introduced:**
```rust
pub fn resolve_cast_mcp_url(
    explicit: Option<String>,
    config: &config::ClientConfig,
) -> Option<String>

pub fn build_server_map(
    cast_url: Option<String>,
    config: &config::ClientConfig,
) -> HashMap<String, config::RemoteServerConfig>
```

**Files:** `src/lib.rs`

---

### S3 — McpClient with custom headers

**Adds:** Updated `McpClient::connect` that accepts a `&RemoteServerConfig` and
forwards its headers to the rmcp transport.

**Behaviors tested:**
- `test_headers_are_sent_to_server` (integration test): a server entry with a
  custom header is configured; the mock axum server uses a middleware layer to
  capture the header; the test asserts the header value was received

**Implementation note:** Header values are converted via
`http::HeaderName::from_str` / `http::HeaderValue::from_str`. Invalid header
names/values return an `anyhow::Error`.

**Signature change:**
```rust
// Before:
pub async fn connect(url: &str) -> anyhow::Result<Self>

// After:
pub async fn connect(server: &config::RemoteServerConfig) -> anyhow::Result<Self>
```

**Files:** `src/lib.rs`, `tests/mcp_client_test.rs`

---

### S4 — CLI + command wiring

**Adds:** Config loading wired into `main.rs`; command function signatures updated;
`--url` renamed to `--cast-mcp-url`; all existing integration tests updated.

**Changes:**
- `main.rs`: `config::load()` called at startup; `cast_mcp_url: Option<String>`
  threaded through all three `Commands` variants; `--url` → `--cast-mcp-url`
- `lib.rs`: command function signatures become:
  ```rust
  pub async fn list_tools_cmd(cast_mcp_url: Option<String>, config: &ClientConfig) -> anyhow::Result<()>
  pub async fn describe_tool_cmd(tool_ref: String, cast_mcp_url: Option<String>, config: &ClientConfig) -> anyhow::Result<()>
  pub async fn call_tool_cmd(tool_ref: String, params: Option<String>, cast_mcp_url: Option<String>, config: &ClientConfig) -> anyhow::Result<()>
  ```
- `tests/mcp_client_test.rs`: all 8 occurrences of `"--url"` renamed to
  `"--cast-mcp-url"`; all existing tests must still pass at end of this slice

**Note:** At this point the commands internally still use the old single-server
logic (temporarily plumbed through), so existing tests remain green. The actual
multi-server and `server/tool` behaviours are introduced in S5–S7.

**Files:** `src/main.rs`, `src/lib.rs`, `tests/mcp_client_test.rs`

---

### S5 — list: multi-server grouped output

**Adds:** Full rewrite of `list_tools_cmd` with concurrent multi-server execution
and grouped JSON output.

**Behaviors tested (new integration tests):**
- `test_list_empty_config_returns_empty_object`: no servers configured (empty
  server map) → stdout is exactly `{}`
- `test_list_grouped_output_single_server`: `CAST_MCP_URL` env injects one mock
  server as the `"cast"` entry → stdout is
  `{ "cast": { "status": "ok", "tools": [...] } }`

**Output contract:**
```json
{
  "server_name": {
    "status": "ok",
    "tools": [...]
  }
}
```

**Files:** `src/lib.rs`, `tests/mcp_client_test.rs`

---

### S6 — list: unreachable server

**Adds:** Error path in the concurrent list — bad URLs produce `"unreachable"`
entries instead of failing the whole command.

**Behaviors tested:**
- `test_list_includes_unreachable_server`: config file (via temp dir) defines two
  servers — one pointing at the mock server, one at an invalid URL; the good
  server shows `"status": "ok"`, the bad server shows `"status": "unreachable"`
  with a non-empty `"error"` string; exit code is 0

**Output contract for error case:**
```json
{
  "server_name": {
    "status": "unreachable",
    "error": "...",
    "tools": []
  }
}
```

**Files:** `src/lib.rs`, `tests/mcp_client_test.rs`

---

### S7 — describe/call: server/tool format + error cases

**Adds:** Full rewrite of `describe_tool_cmd` and `call_tool_cmd` to parse the
`server/tool` reference format, validate inputs, and route to the named server.

**Behaviors tested (new integration tests):**
- `test_describe_server_slash_tool_format`: `describe myserver/dummy_tool` with
  `CAST_MCP_URL` set as `"myserver"` → returns the tool's JSON object; exit 0
- `test_call_server_slash_tool_format`: `call myserver/dummy_tool '{"message":"hi"}'`
  with `CAST_MCP_URL` set as `"myserver"` → returns result JSON; exit 0
- `test_describe_no_separator_fails`: `describe dummy_tool` (no slash) → non-zero
  exit; stderr JSON error message contains `'server/tool'`
- `test_describe_unknown_server_fails`: `describe ghost/dummy_tool` with no server
  named `"ghost"` configured → non-zero exit; stderr JSON error mentions `'ghost'`

**Parsing logic:**
```rust
let (server_name, tool_name) = tool_ref
    .split_once('/')
    .ok_or_else(|| anyhow::anyhow!(
        "tool reference must be in 'server/tool' format, got: '{}'", tool_ref
    ))?;
let server = map.get(server_name).ok_or_else(|| anyhow::anyhow!(
    "server '{}' is not configured", server_name
))?;
```

**Note on test approach for server injection:** Since `describe` and `call` route
by server name from the map (not by URL flag), tests inject the server via
`CAST_MCP_URL` env var, which `build_server_map` inserts under `"cast"`. The
tool ref becomes `cast/dummy_tool`. For tests that need a custom name, a temp
config file is written to a temp working dir (same pattern as S6).

**Files:** `src/lib.rs`, `tests/mcp_client_test.rs`
