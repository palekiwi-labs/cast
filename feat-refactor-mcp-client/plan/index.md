# Master Plan: cast-mcp-client refactor

## Problem

`crates/cast-mcp-client/src/lib.rs` is a 1001-line monolith mixing seven
distinct concerns. `main.rs` owns both Clap structs and dispatch. The crate
has no domain modules. This contrasts sharply with `crates/cast/`, which uses
a clean domain-driven layout.

## Approach

Mechanically redistribute existing code into a module hierarchy that mirrors
`cast`'s structure. No logic changes. The refactor is purely cosmetic/structural.

## Target structure

```
crates/cast-mcp-client/src/
├── lib.rs               # ~9 lines: pub mod + re-exports
├── main.rs              # ~20 lines: parse + commands::run()
│
├── commands/
│   ├── mod.rs           # pub use cli::{Cli, run}
│   ├── cli.rs           # all Clap structs + run() dispatch (from main.rs)
│   ├── call.rs          # call_tool_cmd + read_params
│   ├── describe.rs      # describe_tool_cmd
│   ├── generate.rs      # generate_scripts_cmd
│   ├── list.rs          # list_tools_cmd
│   └── status.rs        # status_cmd
│
├── client/
│   ├── mod.rs           # pub use client::McpClient
│   ├── client.rs        # McpClient struct
│   └── handler.rs       # McpClientHandler
│
├── config/
│   ├── mod.rs           # re-exports
│   ├── schema.rs        # ClientConfig, RemoteServerConfig structs
│   └── loader.rs        # parse_from_str, load, load_from_files, merge,
│                        #   apply_env_substitution
│
├── generate/
│   ├── mod.rs           # pub use script::generate_script
│   ├── params.rs        # ParamSpec, parse_params, camel_to_kebab
│   └── script.rs        # generate_script
│
└── server_map.rs        # build_server_map
```

## Source mapping

| Current location (lib.rs lines) | Destination |
|---|---|
| `build_server_map` (L18-58) | `server_map.rs` |
| `McpClientHandler` (L62-72) | `client/handler.rs` |
| `McpClient` (L74-141) | `client/client.rs` |
| `list_tools_cmd` (L143-208) | `commands/list.rs` |
| `describe_tool_cmd` (L210-238) | `commands/describe.rs` |
| `status_cmd` (L251-292) | `commands/status.rs` |
| `unix_now` (L298-305) | `generate/script.rs` (private) |
| `ParamSpec` (L307-323) | `generate/params.rs` |
| `parse_params` (L325-382) | `generate/params.rs` |
| `generate_script` (L392-513) | `generate/script.rs` |
| `generate_scripts_cmd` (L558-682) | `commands/generate.rs` |
| `print_json_error` (L684-696) | `commands/mod.rs` (or `commands/cli.rs`) |
| `call_tool_cmd` (L698-720) | `commands/call.rs` |
| `read_params` (L722-761) | `commands/call.rs` |
| `#[cfg(test)] mod tests` | split to co-located tests in each module |
| main.rs Clap structs + dispatch | `commands/cli.rs` |

## Public API re-exports in lib.rs

```rust
pub mod client;
pub mod commands;
pub mod config;
pub mod generate;
pub mod server_map;

// Preserve existing public surface for integration tests
pub use client::McpClient;
pub use generate::script::generate_script;
pub use server_map::build_server_map;
pub use commands::print_json_error;
```

## Key design decisions

- `print_json_error` goes in `commands/mod.rs` (it is a CLI output helper,
  not domain logic)
- `read_params` stays alongside `call_tool_cmd` in `commands/call.rs`
  (they are always used together)
- `unix_now` becomes private to `generate/script.rs` (it is only used there)
- Unit tests move to the file that owns the code they test; integration tests
  in `tests/mcp_client_test.rs` are unchanged

## Phases

- [x] Phase 1: Create `config/` module (split existing `config.rs`)
- [x] Phase 2: Create `client/` module (handler + client)
- [x] Phase 3: Create `generate/` module (params + script)
- [x] Phase 4: Create `server_map.rs`
- [x] Phase 5: Create `commands/` module (cli + one file per command)
- [x] Phase 6: Slim down `lib.rs` and `main.rs`
- [x] Phase 7: Verify — `cargo test`, `cargo clippy`
