# cast Enhancement Ideas

This document tracks potential enhancements and features that could be added to cast in the future.

## Configuration Enhancements

### `cast config init`
Generate a sample `cast.json` file to help new users get started.

**Benefits:**
- Easy onboarding for new users
- Can include commented examples of common configurations
- Could have interactive prompts or templates (e.g., `--template rust`, `--template nix`)

**Implementation:**
- Add `Init` subcommand to `ConfigCommands`
- Create example configs as const strings or templates
- Write to `./cast.json` with user confirmation if file exists

---

### `cast config debug`
Validate and debug all components of the config (`cast.json`, env vars) syntax and field values.

**Benefits:**
- Catch configuration errors early
- Provide helpful error messages
- Could be run in CI/CD pipelines

**Implementation:**
- Try to load config and report detailed errors
- Add field-level validation:
  - `memory`: Match pattern `\d+[kmg]`
  - `cpus`: Must be positive number
  - `pids_limit`: Must be positive integer
  - `ports`: Validate port ranges
- Exit with non-zero code on validation failure

---

### `cast config path`
Show which config files are being used and their status.

**Benefits:**
- Help users debug config issues
- Show the config hierarchy clearly
- Indicate which files exist/don't exist

**Example output:**
```
Global config:  ~/.config/cast/cast.json [not found]
Project config: ./cast.json [found, 245 bytes]
Environment:    2 variables set (CAST_MEMORY, CAST_CPUS)
```

**Implementation:**
- Check file existence for global and project configs
- List all `CAST_*` environment variables
- Add `Path` subcommand to `ConfigCommands`

---

### `cast config show --sources`
Show where each configuration value came from (precedence visibility).

**Benefits:**
- Debug configuration merging
- Understand which source is providing each value
- Helpful for troubleshooting

**Example output:**
```json
{
  "memory": "8g",           // from: CAST_MEMORY (env)
  "cpus": 2.0,              // from: ./cast.json (project)
  ...
}
```

**Implementation:**
- Would require tracking metadata during figment merge
- Might need custom Provider wrapper to track sources
- Add `--sources` flag to `show` command

---

### `cast config show --format <format>`
Output configuration in different formats beyond JSON.

**Benefits:**
- Support different user preferences
- Allow config to be piped to other tools
- TOML/YAML might be more human-readable

**Supported formats:**
- `json` (default, current behavior)
- `yaml` (requires `serde_yaml` dependency)
- `toml` (requires `toml` dependency)

**Implementation:**
- Add `--format` flag to `show` command
- Add conditional dependencies for serialization
- Use match on format to select serializer

---

## General Implementation Notes

- These enhancements are **not required** for the core functionality
- They should be implemented **if users request them** or as the tool matures
- Each feature should have its own test coverage
- Documentation should be updated when features are added
