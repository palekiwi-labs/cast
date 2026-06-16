# Plan: Allow Starting Shell in Devshell

## Goal
Modify `cast shell <agent>` to start inside a Nix devshell by default if flakes are detected and enabled, matching the behavior of `cast run`.

## Strategy
1.  **Refactor Flake Detection**: Extract flake detection logic from `run_agent` in `run.rs` into a shared helper in `run.rs` or `utils.rs`.
2.  **Update CLI Definition**: Update `ShellAgent` in `cli.rs` to include a `--raw` flag for opt-out.
3.  **Update `shell` function**: Update `shell.rs` to use `build_command` to wrap the shell command.
4.  **Update CLI Dispatch**: Connect the parsed `raw` flag to the `shell` function.

## Verification
- Unit tests for new logic if applicable.
- Manual verification of `cast shell` and `cast shell --raw` behavior.
