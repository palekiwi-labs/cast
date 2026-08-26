---
title: Support cast.local.json config overrides
status: complete
priority: high
kind: build
tag: 0.2.0
---
# Problem

Most projects git track their `cast.json`, which makes it the natural place
for shared, team-wide container/shell settings. Today the only override
escape hatch for an individual user is environment variables (`CAST_*`),
which works well with `.envrc`/direnv but is awkward without it.

There is no file-based way for a user to keep personal overrides next to
the project - for example `extra_data_volumes` entries that point at
directories that only exist on the user's own machine, which do not make
sense in a tracked `cast.json`.

# Proposal

Support an optional `cast.local.json` in the project root (next to
`cast.json`). It merges on top of the project `cast.json` with higher
precedence, giving users a file-based override layer for git-tracked
settings.

Expected behavior:

- If `cast.local.json` is absent, behavior is unchanged.
- If present, its values override `cast.json` values; keys absent from it
  fall through to `cast.json` and the rest of the existing chain.
- Existing precedence above the project config is preserved:
  `cast-mcp.json` (MCP-specific) and `CAST_*` env vars still win where
  applicable; global config and defaults still lose to it.
- Resulting chain: defaults < global `cast.json` < project `cast.json` <
  `cast.local.json` < `cast-mcp.json` < env vars.
- Merging follows existing figment semantics (deep merge per key; arrays
  replace, as demonstrated by current `env_passthrough` tests).
- Missing or malformed `cast.local.json` handling should match how
  `cast.json`/`cast-mcp.json` are treated today.

# Scope

- `crates/cast/src/config/loader.rs` (`load_config_with_global`): insert the
  new `Json::file(base_dir.join("cast.local.json"))` layer in the figment
  merge stack.
- Tests in `crates/cast/tests/config_test.rs` and/or loader unit tests:
  - `cast.local.json` overrides `cast.json` scalar settings.
  - `cast.json` values apply when `cast.local.json` is silent on a key.
  - No behavior change when the file is absent.
  - Precedence vs env vars and `cast-mcp.json` preserved.
- Docs: mention `cast.local.json` in the cast crate config documentation
  (precedence list in `load_config` doc comment and
  `crates/cast/docs/README.md`-linked config docs), including the
  recommendation to gitignore `cast.local.json`.

# Notes

- Projects are expected NOT to git track `cast.local.json`; it is the
  user-local override file (analogous to `.envrc.local` conventions).
- Implementation should follow TDD: one precedence behavior per red-green
  cycle, starting with the tracer bullet (override scalar from
  `cast.local.json`).
