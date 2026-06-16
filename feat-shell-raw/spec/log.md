# Project Log

## [60c0bcb] Implemented devshell-by-default for cast shell

- **Found:** Flake detection logic was duplicated-to-be-extracted from run.rs
- **Decided:** Make cast shell start in devshell by default if flakes are detected
- **Decided:** Use --raw flag to opt-out of devshell wrapping

## [705daa2] Moved --raw flag to shell parent command

Moved the --raw flag from individual ShellAgent variants to the parent Commands::Shell variant. This enables the intended usage `cast shell --raw <agent>`.

Verified with tests:
- `cast shell --help` now shows the --raw option.
- `cast shell --raw opencode --help` is valid.
- `cast shell opencode --raw` is now rejected as an unexpected argument (since it's no longer on the subcommand).

- **Found:** Current implementation had --raw on subcommands, which was redundant and didn't support the preferred CLI syntax.
- **Decided:** Move --raw flag to parent command for better UX and consistency with intended usage.

## [bf2eb98] Documented shell devshell wrapping and --raw flag

Updated user-facing documentation to reflect the shell devshell-by-default behavior and the --raw flag.

- `commands/reference.md`: Rewrote the `shell` section to document devshell-by-default behavior, the `--raw` flag, correct flag position, and usage examples.
- `nix/flake-integration.md`: Added cross-reference noting the wrapping also applies to `cast shell`.
- `nix/overview.md`: Added cross-reference under "Flake Wrapping" section.

- **Decided:** Option 2 (thorough): update primary reference plus cross-references in Nix docs for discoverability.

