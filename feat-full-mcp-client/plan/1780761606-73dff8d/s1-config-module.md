---
status: complete
---

## Foreword

This executive plan covers **Slice S1: Config module** from the master plan at
`plan/index.md`. It is the first slice and has no prerequisites — the codebase
compiles and all existing tests pass before this work begins.

**Starting state:** `cast-mcp-client` has no config file concept. `lib.rs`
contains `resolve_server_url` which hard-codes a default URL.

**Ending state:** A new `src/config.rs` module exists and is wired into `lib.rs`
via `pub mod config`. It provides `ClientConfig`, `RemoteServerConfig`, and a
`load()` function. All new behavior is covered by unit tests inside `config.rs`.
No existing tests are modified and all continue to pass.

**Scope boundary:** This slice only builds the config data layer. It does not
change any command functions, the CLI flag, or the `McpClient::connect` signature
— those are S2 through S7.

**Key design choices baked in:**
- `parse_from_str` is the core parsing primitive; it is tested directly using
  inline JSON strings (no temp files required for most tests).
- `{env:VAR}` substitution is applied eagerly in `parse_from_str` after
  deserialization. Unset vars are replaced with empty string.
- Substitution uses manual string scanning — no `regex` dep needed.
- Missing config files are silently skipped (not an error).
- Malformed config files print a warning to stderr and are skipped (fallback to
  empty / prior state).
- Global path: `$XDG_CONFIG_HOME/cast/cast-mcp-client.json` if `XDG_CONFIG_HOME`
  is set, else `$HOME/.config/cast/cast-mcp-client.json`.
- Project-local path: `./cast-mcp-client.json`.
- Merge strategy: iterate project entries; each one fully replaces the global
  entry of the same name (no deep field merge).

---

## Steps

### Cycle 1 — Minimal parse

- [x] **RED:** Add `#[cfg(test)] mod tests` to the (not-yet-created) `config.rs`.
  Write `test_parse_minimal_config`: call `parse_from_str` with a JSON string
  containing one server entry (`"myserver": { "url": "http://example.com/mcp" }`).
  Assert the returned `ClientConfig` has one entry with the correct URL.
  Confirm it does not compile / tests fail.

- [x] **GREEN:** Create `src/config.rs`. Define:
  ```rust
  #[derive(Debug, Default, serde::Deserialize)]
  pub struct ClientConfig {
      #[serde(default)]
      pub mcp: std::collections::HashMap<String, RemoteServerConfig>,
  }

  #[derive(Debug, Clone, serde::Deserialize)]
  pub struct RemoteServerConfig {
      pub url: String,
      #[serde(default)]
      pub headers: std::collections::HashMap<String, String>,
      #[serde(default = "default_enabled")]
      pub enabled: bool,
  }

  fn default_enabled() -> bool { true }

  pub fn parse_from_str(s: &str) -> ClientConfig {
      serde_json::from_str(s).unwrap_or_else(|e| {
          eprintln!("cast-mcp-client: warning: failed to parse config: {e}");
          ClientConfig::default()
      })
  }
  ```
  Add `pub mod config;` to `src/lib.rs`. Confirm `test_parse_minimal_config`
  passes.

### Cycle 2 — Defaults for omitted fields

- [x] **RED:** Write `test_default_enabled_and_headers`: parse a JSON entry that
  omits both `"enabled"` and `"headers"`. Assert `enabled == true` and
  `headers.is_empty() == true`.

- [x] **GREEN:** The `#[serde(default)]` and `default_enabled` attributes from
  Cycle 1 should already satisfy this. Confirm the test passes. If not, adjust
  the serde annotations.

### Cycle 3 — `{env:VAR}` substitution

- [x] **RED:** Write `test_env_var_substitution`: using `std::env::set_var` in
  the test, set `CAST_TEST_TOKEN=secret`. Parse a config where a header value
  is `"Bearer {env:CAST_TEST_TOKEN}"`. Assert the resulting header value is
  `"Bearer secret"`. Also write `test_unset_env_var_becomes_empty`: reference an
  env var that is not set; assert the substituted value is `"Bearer "` (empty
  replacement). Clean up with `std::env::remove_var` in both tests.

- [x] **GREEN:** Add `apply_env_substitution(s: &str) -> String` to `config.rs`:
  ```rust
  fn apply_env_substitution(s: &str) -> String {
      let mut result = s.to_string();
      loop {
          let Some(start) = result.find("{env:") else { break };
          let Some(rel_end) = result[start..].find('}') else { break };
          let var_name = result[start + 5..start + rel_end].to_string();
          let value = std::env::var(&var_name).unwrap_or_default();
          result = format!("{}{}{}", &result[..start], value, &result[start + rel_end + 1..]);
      }
      result
  }
  ```
  Call it over every header value inside `parse_from_str`, after deserialization:
  ```rust
  for server in config.mcp.values_mut() {
      for v in server.headers.values_mut() {
          *v = apply_env_substitution(v);
      }
  }
  ```
  Confirm both new tests pass.

### Cycle 4 — Global + project merge

- [x] **RED:** Write `test_project_overrides_global`: build two `ClientConfig`
  values in memory (or parse two JSON strings) where both define a server named
  `"myserver"` with different URLs. Call a `merge(global, project) -> ClientConfig`
  helper. Assert the returned config uses the project URL, not the global URL.
  Also write `test_merge_adds_project_only_servers`: global has `"serverA"`,
  project has `"serverB"`; merged result has both.

- [x] **GREEN:** Add `fn merge(global: ClientConfig, project: ClientConfig) -> ClientConfig`:
  ```rust
  fn merge(mut global: ClientConfig, project: ClientConfig) -> ClientConfig {
      for (name, server) in project.mcp {
          global.mcp.insert(name, server);
      }
      global
  }
  ```
  Confirm both tests pass.

### Cycle 5 — File loading

- [x] **RED:** Write `test_load_from_files_with_project_override` using real temp
  files:
  ```rust
  use std::io::Write;
  let dir = std::env::temp_dir();
  let global_path = dir.join("cast_test_global.json");
  let project_path = dir.join("cast_test_project.json");
  std::fs::write(&global_path, r#"{"mcp":{"s":{"url":"http://global/mcp"}}}"#).unwrap();
  std::fs::write(&project_path, r#"{"mcp":{"s":{"url":"http://project/mcp"}}}"#).unwrap();
  let config = load_from_files(Some(&global_path), Some(&project_path));
  assert_eq!(config.mcp["s"].url, "http://project/mcp");
  // cleanup
  let _ = std::fs::remove_file(&global_path);
  let _ = std::fs::remove_file(&project_path);
  ```

- [x] **GREEN:** Add `pub fn load_from_files`:
  ```rust
  pub fn load_from_files(
      global: Option<&std::path::Path>,
      project: Option<&std::path::Path>,
  ) -> ClientConfig {
      let read_file = |p: &std::path::Path| -> Option<ClientConfig> {
          match std::fs::read_to_string(p) {
              Ok(s) => Some(parse_from_str(&s)),
              Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
              Err(e) => {
                  eprintln!("cast-mcp-client: warning: could not read {}: {e}", p.display());
                  None
              }
          }
      };
      let global_cfg = global.and_then(read_file).unwrap_or_default();
      let project_cfg = project.and_then(read_file).unwrap_or_default();
      merge(global_cfg, project_cfg)
  }
  ```
  Confirm the test passes.

### Cycle 6 — Missing files are silently skipped

- [x] **RED:** Write `test_missing_files_skipped`: call `load_from_files` with
  paths that do not exist. Assert the result is an empty config
  (`config.mcp.is_empty() == true`) and no panic occurs.

- [x] **GREEN:** The `NotFound` branch in `read_file` from Cycle 5 already handles
  this. Confirm the test passes with no code changes.

### Cycle 7 — Malformed config falls back gracefully

- [x] **RED:** Write `test_malformed_config_falls_back`: write `"not valid json"`
  to a temp file, call `load_from_files` with it as the project path. Assert the
  result is an empty config and the process does not panic.

- [x] **GREEN:** The `parse_from_str` fallback to `ClientConfig::default()` from
  Cycle 1 already handles this. Confirm the test passes with no code changes.

### Final wiring — `load()` public entry point

- [x] Implement `pub fn load() -> ClientConfig` that resolves standard paths and
  delegates to `load_from_files`:
  ```rust
  pub fn load() -> ClientConfig {
      let global = global_config_path();
      let project = std::path::PathBuf::from("cast-mcp-client.json");
      load_from_files(global.as_deref(), Some(&project))
  }

  fn global_config_path() -> Option<std::path::PathBuf> {
      let base = std::env::var("XDG_CONFIG_HOME")
          .map(std::path::PathBuf::from)
          .unwrap_or_else(|_| {
              std::env::var("HOME")
                  .map(|h| std::path::PathBuf::from(h).join(".config"))
                  .unwrap_or_default()
          });
      if base == std::path::PathBuf::default() {
          None
      } else {
          Some(base.join("cast").join("cast-mcp-client.json"))
      }
  }
  ```
  No new test needed — `load()` is a thin wrapper and its behavior in the real
  filesystem is environment-dependent. `load_from_files` is the tested surface.

### Verify and commit

- [x] Run `cargo test -p cast-mcp-client` — all tests (old and new) green
- [x] Run `cargo clippy -p cast-mcp-client -- -D warnings` — no warnings
- [x] Commit: `feat(mcp-client): add config module with loading and env substitution`
