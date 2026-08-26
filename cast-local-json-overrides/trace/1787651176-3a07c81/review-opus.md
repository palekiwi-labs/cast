# Opus diff review

Reviewed `master...build-cast-local-json` at `6c5e70d`.

Verification reported by reviewer:

- `cargo test -p cast --lib config::`: 69 passed.
- `cargo test -p cast --test config_test`: 11 passed.
- `cargo clippy --workspace --all-targets`: clean.
- Missing, empty, and malformed local configuration behavior was exercised manually.

The core implementation and precedence chain were found correct. Findings, ordered by reported severity:

## Medium

1. `crates/cast/docs/mcp/configuration.md:8-11` does not document that
   `cast-mcp.json` outranks the `mcp` section in `cast.local.json`. Add the new
   layer and explain that environment variables are required to override keys
   already present in `cast-mcp.json`.

2. `crates/cast/docs/config/approval.md:8-11` names only a malicious
   `cast.json` in its threat model. Since a repository may track
   `cast.local.json` despite the recommendation, name both files in the
   approval documentation.

3. `crates/cast/src/config/loader.rs:194-198` checks only the outer generic
   anyhow context for malformed local JSON. Assert against the formatted error
   chain and verify that it identifies `cast.local.json`.

## Medium-low

4. The documented flagship `extra_data_volumes` use case lacks a test for
   Figment's additive deep merge of maps. Add a loader test proving project and
   local map entries both survive, and document that maps deep-merge while
   arrays replace.

## Low

5. `crates/cast/src/config/loader.rs:139-156` introduces three positional
   `Option<&str>` parameters of the same type. Document their order or replace
   them with a clearer fixture representation.

6. `crates/cast/tests/config_test.rs:94-100` duplicates command setup already
   available through `cast_with_data_dir`; use or improve the helper.

7. `crates/cast/tests/config_test.rs:276-280` uses the default memory value in
   the project fixture, weakening the reapproval scenario. Use a non-default
   value.

8. `crates/cast/docs/config/overview.md:30-31` recommends modifying tracked
   `.gitignore` but omits the clone-local `.git/info/exclude` alternative.

9. New loader tests inherit ambient `CAST_*` variables, so developer-level
   variables such as `CAST_MEMORY` can override fixtures. Consider a way to
   isolate the environment for deterministic tests.

Non-finding: `cargo fmt --all --check` reportedly fails in files untouched by
this branch, and the reviewer verified those failures pre-exist on `master`.
