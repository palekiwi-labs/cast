# Skill: rust-cli

## Fundamental Principles
- **Separation of Concerns**: CLI orchestration is a thin wrapper over a pure domain library.
- **Test-Driven Development (TDD)**: Prioritize "Pure Logic" extraction to enable high test coverage without complex mocking.
- **Capability-Based Logic**: Pass validated configuration and environment state into domain functions rather than having functions "fetch" their own state.

## Blueprint: Directory Structure
Refactor messy projects toward this standard:
- `src/main.rs`: Trivial entrypoint; parses CLI and calls the lib.
- `src/lib.rs`: The library boundary; defines the public domain API.
- `src/commands/`: Thin handlers that bridge CLI inputs to domain logic.
- `src/domain/`: Pure business logic, data transformations, and core models.
- `tests/`: Integration tests that treat the binary as a black box (using `assert_cmd`).

## The "Thin Handler" Protocol
Handlers in `src/commands/` should be strictly limited to:
1. **Resolving Inputs**: Gathering args, env vars, and config.
2. **Context Setup**: Preparing the execution environment (e.g., current directory, user identity).
3. **Delegation**: Calling a domain function with resolved inputs.
4. **Presentation**: Formatting the result (JSON, tables, colors) and emitting to `stdout`/`stderr`.
5. **Exit Status**: Returning an appropriate `Result` or `ExitCode`.

## Zero-Mock TDD (Pure Logic)
Avoid mocks by extracting logic into functions that transform data.
- **Impure**: `fn build_args() -> Vec<String>` (reads from `std::env`).
- **Pure**: `fn build_args(config: &Config, env_vars: &HashMap<String, String>) -> Vec<String>`.
- **Testing**: Unit test pure functions in the same module using `mod tests`. Verify the *transformation result* rather than the *side effect*.

## Idiomatic Communication
### Error Handling
- Use `anyhow` or `eyre` for binary-facing error propagation.
- **Mandatory Context**: Always use `.context("...")` or `.with_context(|| ...)` when bridging layers to provide user-facing "why" it failed.

### Standard IO
- **`stdout` (println!)**: Data output only (for piping/composition).
- **`stderr` (eprintln!)**: Progress updates, diagnostics, and human-friendly hints.

## Progressive Documentation
Organize documentation to avoid information overload:
1. **Root `README.md`**: High-level "what" and "how to start".
2. **Crate/Module `README.md`**: Conceptual architecture and API overview.
3. **Internal `docs/*.md`**: Technical deep dives into specific subsystems.
