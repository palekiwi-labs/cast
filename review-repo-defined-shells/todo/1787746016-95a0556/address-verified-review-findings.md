---
status: complete
priority: high
---
# Address verified review findings

Recommended before merge:

- [x] Expand leading `~/` shell references to the container user's home before invoking Nix, with regression coverage.
- [x] Treat empty and whitespace-only sandbox/project shell references as absent, with regression coverage.
- [x] Restore the generated flake guard against a live `nixConfig` block.
- [x] Make `config init` file creation atomic and safe from final-component symlinks, with regression coverage where practical.
- [x] Document removal of `global_shell` and `CAST_GLOBAL_SHELL`.
- [x] Remove redundant `config init` dispatch/dead code.
- [x] Derive the sandbox-shell announcement from the same layer-selection logic used to build commands.
- [x] Reuse the shared global configuration path helper.
- [x] Correct stale Dockerfile terminology and first-run initialization comments.
- [x] Apply required formatting.

Explicitly excluded:

- The intentional deletion of `cast-mcp-client.json`.
- Optional file logging for `config init`.
- Optional missing-shell UX guidance.
- Micro-optimization of layer cloning unless naturally resolved by the predicate refactor.
- Additional trust-boundary documentation beyond the existing project-config approval model.
- Continuing initialization after an unexpected filesystem write failure.
