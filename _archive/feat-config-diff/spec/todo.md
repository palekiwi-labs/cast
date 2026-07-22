# Todo: Config Diff Support

Each item is a TDD vertical slice: write a failing test first, then the minimal
implementation to make it pass.

## Prerequisites

- [ ] Add `similar` and `owo-colors` to `crates/cast/Cargo.toml`

- [ ] Add `CAST_DATA_DIR` env var override to `approval_store_path()` in
  `config/approval.rs`

  Follows the same pattern as the existing `CAST_LOG_DIR`. Allows integration
  tests to point the approval store at a temp directory without polluting
  `~/.local/share/cast/`.

## Slices

### Slice 1 — Old approval entries load cleanly without a config snapshot

**Test** (unit, `approval.rs`): Deserialize a raw JSON string representing a
legacy `ApprovalEntry` with no `approved_config` field. Assert deserialization
succeeds and `approved_config` is `None`.

**Implementation:** Add `approved_config: Option<serde_json::Value>` to
`ApprovalEntry` with `#[serde(default, skip_serializing_if = "Option::is_none")]`.

---

### Slice 2 — Approving a config captures a snapshot

**Test** (unit, `approval.rs`): Call `add_entry` with a hash, workspace, and a
`serde_json::Value` snapshot. Save and reload the store from disk. Assert the
reloaded entry's `approved_config` matches the original snapshot.

**Implementation:** Update `add_entry` to accept a `serde_json::Value` argument.
Update `approve_workspace_config` to call `serde_json::to_value(config)` and
pass it through.

---

### Slice 3 — Store can look up an entry by workspace path

**Test** (unit, `approval.rs`): Add an entry for workspace A. Assert
`find_by_workspace(A)` returns `Some`. Assert `find_by_workspace(B)` returns
`None`.

**Implementation:** Add `find_by_workspace(canonical_path: &str) ->
Option<&ApprovalEntry>` to `ApprovalStore`. Re-export via `config/mod.rs`.

---

### Slice 4 — `format_config_diff` returns a text diff of two JSON values

**Test** (unit, `diff.rs`): Create two `serde_json::Value` objects differing by
one field (e.g. `memory` from `"1024m"` to `"2048m"`). Assert the output string
contains a line prefixed with `-` that includes the old value, and a line
prefixed with `+` that includes the new value.

**Implementation:** Create `crates/cast/src/config/diff.rs` with
`format_config_diff(old: &serde_json::Value, new: &serde_json::Value) -> String`.
Use `similar::TextDiff::from_lines` on `serde_json::to_string_pretty` of each
value. Unified diff with `context_radius(3)`. Plain text only — no ANSI codes;
coloring is the caller's responsibility. Declare `mod diff` and re-export in
`config/mod.rs`.

---

### Slice 5 — `cast config diff` exits cleanly when the config is not yet approved

**Test** (integration, `config_test.rs`): Run `cast config diff` in a temp
workspace with `CAST_DATA_DIR` pointing to an empty temp directory. Assert exit
code is success and output contains text indicating the config is not approved
and suggesting `cast config allow`.

**Implementation:** Add `Diff` variant to `ConfigCommands`. Implement the `Diff`
handler; handle the no-entry case.

---

### Slice 6 — `cast config diff` handles a legacy entry that has no snapshot

**Test** (integration, `config_test.rs`): Seed the temp store with a raw JSON
`ApprovalEntry` (no `approved_config` field) matching the current workspace. Run
`cast config diff`. Assert the output explains no snapshot is available and
suggests running `cast config allow`.

**Implementation:** Handle `approved_config: None` in the `Diff` handler.

---

### Slice 7 — `cast config diff` reports no changes when the config is approved and unchanged

**Test** (integration, `config_test.rs`): Run `cast config allow` in a temp
workspace (with `CAST_DATA_DIR` set), then run `cast config diff`. Assert the
output indicates no changes.

**Implementation:** Compute the current config hash, compare against the store
entry key, and print a "no changes" message when they match.

---

### Slice 8 — `cast config diff` shows a diff when the config has changed

**Test** (integration, `config_test.rs`): Write config A to a temp workspace,
run `cast config allow`, overwrite with config B (different `memory` value), run
`cast config diff`. Assert the output contains the old value on a `-` line and
the new value on a `+` line.

**Implementation:** Call `format_config_diff` in the `Diff` handler and print
the result with `owo-colors` coloring (red for `-`, green for `+`, dim for
context).

---

### Slice 9 — `cast config show` hints at `cast config diff` when the config is unapproved

**Test** (integration, `config_test.rs`): Run `cast config show` in a temp
workspace with no approval (`CAST_DATA_DIR` pointing to an empty temp dir).
Assert stdout is still valid JSON (stdout contract preserved). Assert stderr
contains a mention of `cast config diff`.

**Implementation:** In the `Show` handler, load the store, compute the current
hash, and emit a one-line hint to stderr if the hash is not approved. Also
update the `anyhow::bail!` message in `ApprovalStore::verify` to mention
`cast config diff`.
