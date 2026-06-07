# Project Log

## [566076b] Refactor: Replace reqwest with ureq in cast crate

Refactored the `cast` crate to use `ureq` instead of `reqwest` for version fetching.
Reduced binary bloat by removing transitive `reqwest` dependency via `jsonschema` (disabled default-features).
This aligns with the decision to use `reqwest` only where async/streaming is required (now in the separate `cast-mcp-client` crate), while keeping the main `cast` CLI lightweight.

Verification:
- `cargo check --features mcp -p cast` succeeded.
- `cargo tree -p cast --features mcp | grep reqwest` confirms removal from `cast` tree.
- All tests in `crates/cast` passed.

- **Found:** reqwest was pulled in transitively by jsonschema default features
- **Found:** ureq is sufficient for simple blocking version fetching in cast CLI
- **Decided:** Replace reqwest with ureq in cast binary to reduce bloat
- **Decided:** Disable jsonschema default features to remove transitive reqwest dependency

