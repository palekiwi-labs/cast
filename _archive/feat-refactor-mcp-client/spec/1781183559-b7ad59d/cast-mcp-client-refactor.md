# cast-mcp-client Refactor Proposal

## Status: proposal (not yet implemented)

## Problem

`cast-mcp-client/src/lib.rs` is a 1001-line monolith that mixes seven
distinct concerns in a single file. `cast` uses a clean, domain-driven
module hierarchy that scales well. This document captures the analysis
and the proposed directory/file structure.

---

## Pattern Analysis: `cast`

### Structural principles

| Principle | How `cast` applies it |
|---|---|
| **Thin `main.rs`** | 18 lines — just `Cli::parse()` + `run()` |
| **Thin `lib.rs`** | 9 lines — pure `pub mod` declarations |
| **`commands/` façade** | One file per subcommand group; `mod.rs` re-exports `Cli` and `run` |
| **`commands/cli.rs`** | Owns all Clap structs + the top-level `run()` dispatch |
| **Domain modules** | `config/`, `dev/`, `mcp/`, `docker/`, `nix_daemon/`, `user/` — each with a `mod.rs` that re-exports its public API |
| **`commands/X.rs` is a thin adapter** | e.g. `commands/mcp.rs` = 14 lines, just calls `crate::mcp::server::run_http_server(...)` |
| **Domain logic isolated** | Each domain sub-module (`mcp/server.rs`, `mcp/handler.rs`, `mcp/exec.rs`, `mcp/docs.rs`) owns exactly one concern |
| **Sub-domains get sub-directories** | `dev/claudecode/`, `dev/opencode/`, `dev/pi/`, `dev/version/` |
| **No business logic in `main.rs`** | All logic goes through `lib.rs` → `commands::run()` |

---

## Problem Analysis: `cast-mcp-client`

### What lives in `lib.rs` (1001 lines)

| Lines | Concern |
|---|---|
| 1–58 | `build_server_map` — config + routing logic |
| 60–141 | `McpClientHandler` + `McpClient` struct (transport layer) |
| 143–208 | `list_tools_cmd` — command implementation |
| 210–238 | `describe_tool_cmd` — command implementation |
| 240–292 | `status_cmd` — command implementation |
| 294–513 | `generate_script` + `parse_params` + `camel_to_kebab` (codegen domain) |
| 514–682 | `generate_scripts_cmd` — command implementation |
| 684–761 | `call_tool_cmd` + `read_params` — command implementation + I/O helper |
| 684–696 | `print_json_error` — error formatting utility |
| 763–1001 | Unit tests mixed with all of the above |

### What lives in `main.rs` (143 lines)

Contains the full Clap struct definitions AND the dispatch match — command
structs and dispatch are in the same file instead of being separated.

---

## Proposed Structure

```
crates/cast-mcp-client/src/
├── lib.rs                    # 9 lines: pub mod declarations only
├── main.rs                   # ~20 lines: parse + run()
│
├── commands/
│   ├── mod.rs                # pub use cli::{Cli, run}
│   ├── cli.rs                # Clap structs + run() dispatch (from main.rs)
│   ├── list.rs               # list_tools_cmd  (from lib.rs ~143-208)
│   ├── describe.rs           # describe_tool_cmd  (from lib.rs ~210-238)
│   ├── call.rs               # call_tool_cmd + read_params  (from lib.rs ~698-761)
│   ├── status.rs             # status_cmd  (from lib.rs ~251-292)
│   └── generate.rs           # generate_scripts_cmd  (from lib.rs ~558-682)
│
├── client/
│   ├── mod.rs                # pub use client::McpClient; pub use handler::McpClientHandler
│   ├── client.rs             # McpClient struct: connect/call_tool/list_tools/shutdown
│   └── handler.rs            # McpClientHandler (ClientHandler impl)
│
├── config/
│   ├── mod.rs                # pub use schema::{ClientConfig, RemoteServerConfig}; pub use loader::load
│   ├── schema.rs             # ClientConfig, RemoteServerConfig structs + serde
│   └── loader.rs             # parse_from_str, load, load_from_files, merge, apply_env_substitution
│
├── generate/
│   ├── mod.rs                # pub use script::generate_script; pub use params::parse_params
│   ├── script.rs             # generate_script fn (bash codegen)
│   └── params.rs             # ParamSpec, parse_params, camel_to_kebab
│
└── server_map.rs             # build_server_map fn (routing / config bridge)
    # OR: fold into config/mod.rs since it bridges config → HashMap
```

### Alternative: fold `server_map` into `config/`

`build_server_map` takes a `&ClientConfig` and produces
`HashMap<String, RemoteServerConfig>`. It belongs in `config/` as
`config::build_server_map` — it is config-resolution logic, not a command.

---

## Migration notes

- `lib.rs` becomes 9 lines of `pub mod` (mirrors `cast/src/lib.rs`)
- `main.rs` becomes ~20 lines: just `Cli::parse()` → `commands::run()`
- All Clap structs move from `main.rs` → `commands/cli.rs`
- Each `*_cmd` fn moves to its own file under `commands/`
- `McpClient` + `McpClientHandler` move to `client/`
- `generate_script` + `parse_params` + `camel_to_kebab` + `ParamSpec` move to `generate/`
- Existing `config.rs` splits into `config/schema.rs` + `config/loader.rs`
- Unit tests stay co-located with their source (e.g. `config/loader.rs` keeps config tests, `generate/params.rs` keeps `camel_to_kebab` tests)
- Integration tests in `tests/mcp_client_test.rs` are unchanged — public API stays identical

---

## Public API impact

Zero breaking changes. All currently-public items remain public through
their new module path or via re-exports in `lib.rs`:

```rust
// lib.rs after refactor
pub mod client;
pub mod commands;
pub mod config;
pub mod generate;
pub mod server_map;

// keep these re-exports for backward compat with tests/external callers:
pub use client::McpClient;
pub use generate::script::generate_script;
pub use server_map::build_server_map;
pub use commands::print_json_error;
```
