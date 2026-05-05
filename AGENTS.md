This is an ongoing Rust rewrite of the original `ocx` application prototype written
in nushell. This project is now named `cast`.

Refences:
- local clone of ocx repo: `.mem/master/ref/repos/palekiwi-labs/ocx`
- preliminary specification: `.mem/master/spec/ocx-spec.md`

The rewrite is not intended to be a 1:1 clone, for example only the "nix flow"
will be implemented (nix daemon and dev containers).

Load the "tdd" skill - we will apply TDD techniques as far as possible in this rewrite.
Load the "git-commit" skill for commit standards.

Support the following platforms only:
- Linux x86
- MacOS arm

### Testing Guidelines
- **Nix Compatibility**: All tests must be written to conform to `nix build` reproducible build constraints.
  - Tests run in a sandboxed, read-only environment without network access.
  - Do not assume access to `$HOME` or global state (like `~/.local/share`).
  - When executing subprocesses or accessing the filesystem during tests, explicitly route outputs and side effects to temporary directories (e.g., via `std::env::temp_dir()`).
