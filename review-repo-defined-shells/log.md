# Project Log

## [32f74a0] Implemented verified shell review fixes

Committed the recommended pre-merge fixes as 32f74a0. Shell refs now expand leading `~/` against the resolved container username, empty refs are ignored consistently, config initialization uses atomic create-new semantics, and review-driven tests and migration documentation were restored.

- **Found:** Direct Docker argv requires cast to expand container-home shell refs explicitly
- **Found:** The same effective-layer helper can drive both command construction and devshell announcements
- **Found:** Atomic create-new prevents following a dangling final-component symlink
- **Decided:** Expand leading `~/` for both sandbox and project shell refs
- **Decided:** Treat empty and whitespace-only refs as absent
- **Decided:** Exclude intentional cast-mcp-client.json deletion and optional UX/logging items

