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
| `list` output | Flat array of tools: `[ { "name": "server/tool", ... } ]` — ideal for AI agents |
| Tool reference format | `server/tool` — always required, no bare-name shorthand |
| `--url` rename | `--cast-mcp-url` (makes cast-specific purpose explicit) |
| Config locations | `~/.config/cast/cast-mcp-client.json` (global) + `./cast-mcp-client.json` (project, wins on conflict) |
| Multi-server resolution | All `enabled: true` config entries + optional `"cast"` entry from flag/env |
| Bad server behaviour (list) | Warning to stderr, silently skipped from output, never blocks other servers |
| Malformed config | Warning to stderr, silently skipped, falls back to `Default` |
| Server Diagnostics | Handled by a dedicated `status` command showing server state / health |

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

### Concurrency in list & status

Both `list` and `status` commands contact all servers concurrently using `futures::future::join_all`.
Each future resolves to `(name, Result<...>)`. Errors are handled on a per-server basis
and do not abort the command or block other healthy servers.

---

## Breaking Changes

- `list` output remains a flat JSON array `[Tool]`, but each tool's `name` is now prefixed as `"server_name/tool_name"`.
- `describe` and `call` now require `server/tool` format instead of bare `tool`.
  Existing callers (if any) must be updated.
- `--url` flag is removed; replaced by `--cast-mcp-url`.

---

## Implementation Slices

Work is divided into eight vertical TDD slices. Each slice is independently
committable: tests are written first (RED), then implementation (GREEN).

### S1 — Config module [x]

**Adds:** `src/config.rs` (new file), `pub mod config` in `src/lib.rs`.
Completed and verified with unit tests.

---

### S2 — Server map logic [ ]

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

### S3 — McpClient with custom headers [ ]

**Adds:** Updated `McpClient::connect` that accepts a `&RemoteServerConfig` and
forwards its headers to the rmcp transport.

**Behaviors tested:**
- `test_headers_are_sent_to_server` (integration test): a server entry with a
  custom header is configured; the mock axum server uses a middleware layer to
  capture the header; the test asserts the header value was received

**Files:** `src/lib.rs`, `tests/mcp_client_test.rs`

---

### S4 — CLI + command wiring [ ]

**Adds:** Config loading wired into `main.rs`; command function signatures updated;
`--url` renamed to `--cast-mcp-url`; all existing integration tests updated.

**Changes:**
- `main.rs`: `config::load()` called at startup; `cast_mcp_url: Option<String>`
  threaded through all commands; `--url` → `--cast-mcp-url`.
- `lib.rs`: command function signatures updated.
- `tests/mcp_client_test.rs`: all occurrences of `"--url"` renamed to `"--cast-mcp-url"`.

---

### S5 — list: multi-server flat prefixed output [ ]

**Adds:** Full rewrite of `list_tools_cmd` to gather tools concurrently from all configured
servers and prefix each tool's name with `"{server_name}/"`.

**Behaviors tested:**
- `test_list_empty_config_returns_empty_array`: no servers configured → stdout is exactly `[]`
- `test_list_prefixed_tools_single_server`: single server with tool `dummy_tool` outputs `[{"name":"cast/dummy_tool",...}]`

**Files:** `src/lib.rs`, `tests/mcp_client_test.rs`

---

### S6 — list: handle unreachable servers gracefully [ ]

**Adds:** Concurrent list error capturing. Unreachable servers print warnings to stderr,
but are skipped from stdout without causing the command to fail.

**Behaviors tested:**
- `test_list_ignores_unreachable_server`: one good + one unreachable server → stdout lists only the good server's tools, stderr prints warning, exit code is 0.

**Files:** `src/lib.rs`, `tests/mcp_client_test.rs`

---

### S7 — status command [ ]

**Adds:** A new `status` CLI command that performs concurrent server health checks
and displays diagnostic JSON of all configured servers.

**Behaviors tested:**
- `test_status_command_output`: verifies JSON format of both reachable and unreachable servers.

**Output Schema:**
```json
{
  "cast": {
    "status": "ok",
    "url": "http://127.0.0.1:8080/mcp",
    "tools_count": 1
  },
  "sentry": {
    "status": "unreachable",
    "url": "https://mcp.sentry.dev/mcp",
    "error": "..."
  }
}
```

**Files:** `src/main.rs`, `src/lib.rs`, `tests/mcp_client_test.rs`

---

### S8 — describe/call: server/tool format + error cases [ ]

**Adds:** Full rewrite of `describe_tool_cmd` and `call_tool_cmd` to parse `server/tool` references
and route to the designated server.

**Behaviors tested:**
- `test_describe_server_slash_tool_format`
- `test_call_server_slash_tool_format`
- `test_describe_no_separator_fails`
- `test_describe_unknown_server_fails`

**Files:** `src/lib.rs`, `tests/mcp_client_test.rs`
