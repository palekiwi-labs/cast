---
status: complete
---
# Fix: cast server config headers silently stripped

## Foreword

The critical bug is in the interface between `resolve_cast_mcp_url` and
`build_server_map`. The functions are individually correct, but `main.rs` wires
them together incorrectly.

`resolve_cast_mcp_url` has three fallback levels:
1. CLI flag (`explicit`)
2. Env var (`env_url`)
3. Config file (`config.mcp["cast"].url`)

It always returns the first `Some(url)` it finds — including the config fallback
(level 3). That return value is then passed as `cast_url` to `build_server_map`,
which treats **any** `Some(url)` as "came from flag/env" and constructs a bare
entry with `headers: {}` and `enabled: true`, silently discarding the config's
custom headers and ignoring `enabled: false`.

The existing unit test for the config-preserving path
(`test_build_server_map_preserves_full_cast_entry_when_url_from_config`) passes
only because it calls `build_server_map(None, &cfg)` directly, bypassing
`resolve_cast_mcp_url` entirely.

### Root cause

`main.rs` passes the result of `resolve_cast_mcp_url(flag, env, &cfg)` to
`build_server_map`, but `build_server_map` needs to distinguish between
"URL from flag/env" (levels 1-2) and "URL from config" (level 3). By always
resolving through `resolve_cast_mcp_url`, that distinction is lost.

### Fix strategy

Split the resolution in `main.rs` into two steps:

1. Compute the **override** (levels 1-2 only): `flag.or(env_url)`.
2. Pass that override (which is `None` when neither flag nor env is set) to
   `build_server_map`.

`build_server_map` already handles the config path correctly when it receives
`None` — it looks up `config.mcp["cast"]` directly, respecting headers and
`enabled`. No changes to `lib.rs` are needed.

`resolve_cast_mcp_url` can be deleted (or kept for other callers), but it is no
longer needed in `main.rs`.

---

## Steps

- [ ] Read `main.rs` to confirm current wiring (done — lines 94, 103, 113, 118,
  127 all follow the same pattern).

- [ ] In `main.rs`, for every command arm, replace:
  ```rust
  let cast_url = resolve_cast_mcp_url(cast_mcp_url, env_url, &cfg);
  let server_map = build_server_map(cast_url, &cfg);
  ```
  with:
  ```rust
  let cast_override = cast_mcp_url.or(env_url.clone());
  let server_map = build_server_map(cast_override, &cfg);
  ```
  Note: `env_url` must be cloned on each arm since it is used multiple times.
  Alternatively, clone `env_url` once up-front and use the clone in each arm.

- [ ] Remove the `resolve_cast_mcp_url` import from `main.rs` (it will no
  longer be called from `main.rs`).

- [ ] Check whether `resolve_cast_mcp_url` still has callers anywhere in the
  codebase. If it has no other callers, delete it from `lib.rs` and its unit
  tests. If it does, leave it in place.

- [ ] Run the unit tests to verify the existing
  `test_build_server_map_preserves_full_cast_entry_when_url_from_config` test
  still passes — it should, since `build_server_map` is unchanged.

- [ ] Add a new integration test (or update an existing one) that:
  - Writes a `cast-mcp-client.json` with a `"cast"` entry that has a custom
    header and `enabled: true`.
  - Sets neither `--cast-mcp-url` flag nor `CAST_MCP_URL` env var.
  - Verifies the header is actually sent to the mock server.
  This confirms the fix end-to-end through `main.rs` rather than just lib.

- [ ] Run the full test suite (`cargo test -p cast-mcp-client`) to confirm all
  tests pass before committing.

- [ ] Commit with message:
  `fix(mcp-client): cast config headers no longer stripped when URL from config`
