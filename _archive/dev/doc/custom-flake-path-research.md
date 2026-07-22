# Research Report: Custom Flake Location Configuration

## Research Question
Does `cast` currently have a way to specify a custom location for the project flake when `use_flake` is enabled?

## Summary
Yes, `cast` already supports specifying a custom flake location via the `use_flake_path` configuration field. However, this feature is currently undocumented.

## Findings

### Configuration Schema
The `use_flake_path` field is defined in the configuration schema and is an optional string.

**File:** `crates/cast/src/config/schema.rs`
```rust
pub use_flake: bool,
#[serde(skip_serializing_if = "Option::is_none")]
pub use_flake_path: Option<String>,
```

### Resolution Logic
The system uses the following logic to determine which flake to use when `use_flake` is enabled:
1. If `use_flake_path` is explicitly set, use that path/reference.
2. If `use_flake_path` is missing, but a `flake.nix` is detected at the workspace root, use `.` (the project root).
3. Otherwise, do not apply a project flake (though a global flake may still be applied if present at `~/.config/cast/nix/flake.nix`).

**File:** `crates/cast/src/dev/build_command.rs`
```rust
// Project flake (inner layer)
if config.use_flake {
    let project_flake = if let Some(path) = &config.use_flake_path {
        Some(path.as_str())
    } else if opts.project_flake_present {
        Some(".")
    } else {
        None
    };

    if let Some(flake_ref) = project_flake {
        cmd.extend([
            "nix".to_string(),
            "develop".to_string(),
            flake_ref.to_string(),
            "-c".to_string(),
        ]);
    }
}
```

### Usage
Users can specify the custom path in their `cast.json` file:

```json
{
  "use_flake": true,
  "use_flake_path": "/path/to/your/flake/directory"
}
```

Or via an environment variable:
`CAST_USE_FLAKE_PATH="/path/to/your/flake/directory"`

## Documentation Gap
This feature is currently not documented in any user-facing READMEs or documentation files. It was found by inspecting the source code and internal research logs.
