---
status: complete
---

# S5 — list: multi-server flat prefixed output

## Foreword

Rewrite `list_tools_cmd` to gather tools from all configured servers concurrently,
prefix each tool name with `"{server_name}/"`, and output a flat JSON array.
The `--server` filter (already wired in S4) is now fully implemented.

Prerequisites: S4 committed (eafa5bc). 31 tests passing.

## Behaviors to test

1. `test_list_empty_config_returns_empty_array` — no servers → stdout is `[]`
2. `test_list_prefixed_tools_single_server` — single server "cast" with dummy_tool → `[{"name":"cast/dummy_tool",...}]`
3. `test_list_filter_by_server` — two servers, `--server sentry` returns only sentry tools
4. `test_list_unknown_server_fails` — `--server ghost` → non-zero exit + COMMAND_ERROR JSON

## Steps

- [ ] RED: write all 4 tests (they share spawn_mock_server helper; tests 3-4 need a second server)
- [ ] GREEN: rewrite list_tools_cmd in lib.rs — concurrent join_all, prefix, filter
- [ ] Verify all 35 tests pass
- [ ] Commit
