## Agent Skills

Always load:

- `tdd`: we will apply TDD techniques as much as practical and possible
- `git-commit`

## Documentation

Docs are organized for progressive discovery: each README is a table of
contents — read it first, then fetch individual entries as needed.

- **Project**: `docs/README.md`
- **`cast` crate**: `crates/cast/docs/README.md`
- **`cast-mcp-client` crate**: `crates/cast-mcp-client/docs/README.md`

## Supported platforms

- Linux x86
- MacOS arm

### Testing Guidelines

- **Nix Compatibility**: All tests must be written to conform to `nix build` reproducible build constraints.
  - Tests run in a sandboxed, read-only environment without network access.
  - Do not assume access to `$HOME` or global state (like `~/.local/share`).
  - When executing subprocesses or accessing the filesystem during tests, explicitly route outputs and side effects to temporary directories (e.g., via `std::env::temp_dir()`).
