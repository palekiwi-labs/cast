---
status: complete
---

## Foreword

Phase 1 of the `generate` command (master plan: `.mem/feat-full-mcp-client/plan/generate-command.md`).
Implements the full end-to-end `generate` subcommand: schema-to-bash-script generation,
CLI wiring, executable output, and JSON result envelope. No resilience handling yet (Phase 2).

Prerequisites: all 38 existing tests pass; `feat/full-mcp-client` branch.

---

## Steps

- [x] Cycle 1 RED: add `test_camel_to_kebab` unit test in lib.rs
- [x] Cycle 1 GREEN: implement `camel_to_kebab` in lib.rs
- [x] Cycle 2 RED: add `test_generate_script_content` unit test in lib.rs
- [x] Cycle 2 GREEN: implement `generate_script` in lib.rs
- [x] Cycle 3 RED: add `test_generate_creates_scripts` integration test
- [x] Cycle 3 GREEN: implement `generate_scripts_cmd` + `Generate` CLI subcommand
- [x] Commit GREEN (cycles 1-3)
- [x] Cycle 4 RED: add `test_generate_script_runs_correctly` integration test
- [x] Cycle 4 GREEN: fix any script template issues surfaced by execution
- [x] Cycle 5 RED: add `test_generate_script_tool_error` integration test
- [x] Cycle 5 GREEN: fix error path if needed
- [x] Final lint + full test run
- [x] Commit all remaining GREEN cycles
