---
status: complete
---

# S4 — CLI + Command Wiring

## Foreword

This plan covers Slice 4 of the cast-mcp-client migration. It wires the config loading
and server-map logic into main.rs, renames the `--url` flag to `--cast-mcp-url`, adds
`--server` to `list`, and updates command signatures and all integration tests accordingly.

Prerequisites: S1, S2, S3 are committed (dcc63ca). 30 tests passing.

The implementation is TDD: integration tests go RED first for each behavior,
then implementation makes them GREEN.

## Steps

### Cycle 1: rename --url to --cast-mcp-url (integration test → impl)

- [ ] RED: Update integration tests — replace all `--url` with `--cast-mcp-url`
- [ ] GREEN: Update main.rs clap definitions to use `--cast-mcp-url`; keep lib.rs command
  functions unchanged for now (still accept `Option<String>` url)
- [ ] Verify tests pass

### Cycle 2: add --server filter to list (integration test → impl)

- [ ] RED: Add `test_list_server_flag_accepted` — call `list --cast-mcp-url <url> --server cast`
  and assert success (the flag is accepted, even if multi-server full support is S5)
- [ ] GREEN: Add `--server` arg to `List` variant in clap; pass it through (stub/ignore for now)
- [ ] Verify tests pass

### Cycle 3: wire config::load() into main.rs

- [ ] RED: Add `test_list_reads_project_config` — write a temp `cast-mcp-client.json` with a
  cast entry pointing to a mock server, run `list` with `current_dir(tmpdir)` and NO `--cast-mcp-url`
  flag, assert it connects and returns tools
- [ ] GREEN: Call `config::load()` in main; compute `cast_url` via `resolve_cast_mcp_url`; for
  now `list_tools_cmd` still uses its url param but we thread the resolved URL through
- [ ] Verify tests pass

### Cycle 4: update command function signatures to accept server map

- [ ] Update `list_tools_cmd`, `describe_tool_cmd`, `call_tool_cmd` in lib.rs to accept
  resolved `url: Option<String>` (no behavioral change — still build bare RemoteServerConfig
  inside; S5/S8 will replace with full server-map logic)
- [ ] Wire in main.rs: `config::load()` → `resolve_cast_mcp_url(flag, env, &cfg)` → pass to cmds
- [ ] Verify all 30+ tests pass

### Commit

- [ ] Commit: feat(mcp-client): S4 — wire config load + rename --url to --cast-mcp-url
