## Agent Skills

Load the following skills:

- `tdd`: we will apply TDD techniques as much as practical and possible
- `git-commit`

## Supported platforms

- Linux x86
- MacOS arm

### Testing Guidelines

- **Nix Compatibility**: All tests must be written to conform to `nix build` reproducible build constraints.
  - Tests run in a sandboxed, read-only environment without network access.
  - Do not assume access to `$HOME` or global state (like `~/.local/share`).
  - When executing subprocesses or accessing the filesystem during tests, explicitly route outputs and side effects to temporary directories (e.g., via `std::env::temp_dir()`).
