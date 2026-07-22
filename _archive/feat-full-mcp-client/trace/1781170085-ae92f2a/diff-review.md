---
status: complete
---
# Branch Diff Review — feat/full-mcp-client

Reviewer: diff-reviewer-gemini-3.5-flash
Diff: .mem/feat-full-mcp-client/tmp/1781170085-ae92f2a/branch.diff

---

## Critical

### cast server config headers silently stripped & disabled state ignored

**Files:** `crates/cast-mcp-client/src/main.rs` (lines ~94–130),
`crates/cast-mcp-client/src/lib.rs` (lines ~18–58)

`resolve_cast_mcp_url` falls back to the config-file URL and returns `Some(url)`
even when no CLI flag or env var was set. That `Some(url)` is passed directly to
`build_server_map`, which interprets any `Some(...)` as "came from flag/env" and
inserts a bare entry with `headers: HashMap::new()` and `enabled: true`, silently
discarding the config's custom headers and ignoring the `enabled: false` flag.

The unit test `test_build_server_map_preserves_full_cast_entry_when_url_from_config`
passes only because it manually bypasses `resolve_cast_mcp_url` and passes `None`.

**Fix:** In `main.rs`, compute `cast_override` as only the CLI flag or env var
result (not the config fallback), and pass `None` when neither is present so that
`build_server_map` reads the full config entry directly.

---

## High

### McpClient not shut down on operation failure

**Files:** `crates/cast-mcp-client/src/lib.rs` (lines ~197–208, ~230–257,
~471–486, ~709–731)

The `?` operator is used after connection in `list_tools_cmd`, `describe_tool_cmd`,
`call_tool_cmd`, and `generate_scripts_cmd`. If the operation errors, the client
is dropped without calling `shutdown()`, potentially leaving Tokio runtime tasks
blocked for ~5 seconds.

**Fix:** Capture operation result first, call `shutdown()` unconditionally, then
propagate the error.

---

### Generated bash scripts crash on missing flag value under set -e

**File:** `crates/cast-mcp-client/src/lib.rs` (lines ~466–474)

`shift 2` is called for every matched `--flag`. Under `set -euo pipefail`, if the
flag is the last argument with no value, `shift 2` fails and terminates the script
with a generic bash error instead of a useful message.

**Fix:** Add a bounds check before `shift 2`:
```bash
if [[ $# -lt 2 ]]; then
  echo "Error: --{{flag}} requires a value" >&2; exit 1
fi
```

---

## Medium

### Unsafe env mutation in config tests races with parallel tests

**File:** `crates/cast-mcp-client/src/config.rs` (lines ~139–152)

`test_env_var_substitution` uses `unsafe { std::env::set_var(...) }` and
`unsafe { std::env::remove_var(...) }`. Parallel test execution can race against
other env-reading tests, causing intermittent failures or undefined behaviour.

**Fix:** Use the `serial_test` crate (`#[serial]`) or a static `Mutex` to
serialise env-sensitive tests.

### Sequential jq invocations in generated scripts

**File:** `crates/cast-mcp-client/src/lib.rs` (lines ~487–505)

One `jq` subprocess per parameter builds the payload sequentially, O(N) process
spawns. Collapse into a single `jq -n '$ARGS.named'` invocation using a bash
array of `--arg`/`--argjson` flags.

### jq crashes on null .content in generated scripts

**File:** `crates/cast-mcp-client/src/lib.rs` (lines ~512–522)

`.content[]` throws `Cannot iterate over null` if the server returns no content
field. Change to `.content[]?` throughout the script template.

---

## Low

### PermissionsExt imported unconditionally

**File:** `crates/cast-mcp-client/src/lib.rs` (lines ~654–655)

`std::os::unix::fs::PermissionsExt` is a Unix-only trait imported without a
`#[cfg(unix)]` guard. Wrap the import and the `set_permissions` call in
`#[cfg(unix)]` for cross-platform portability.
