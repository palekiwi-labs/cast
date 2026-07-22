# Research: Dropping Special Handling of `opencode_config` and `opencode_config_dir`

## Overview
This report investigates the feasibility of removing the specialized handling for `OPENCODE_CONFIG` and `OPENCODE_CONFIG_DIR` environment variables in the `OpenCode` agent module in favor of using the general-purpose `extra_data_volumes` feature in `cast.json`.

## Current Implementation

The specialized logic is primarily located in `src/dev/opencode/mod.rs`. It intercepts the `OPENCODE_CONFIG` and `OPENCODE_CONFIG_DIR` host environment variables to set up bind mounts and rewrite the environment variables inside the container.

### Special Handling Logic
File: `src/dev/opencode/mod.rs`
```rust
// ... inside OpenCode::extra_run_args ...

// OPENCODE_CONFIG_DIR special case: bind-mount with container path rewrite.
let opencode_config_dir_env = resolve_config_dir_env(
    env.get("OPENCODE_CONFIG_DIR").cloned(),
    opts.host_home_dir.as_deref(),
)?;
if let Some(config_dir_env) = &opencode_config_dir_env {
    args.extend([
        "-v".to_string(),
        format!("{}:/opencode-config-dir:ro", config_dir_env.display()),
        "-e".to_string(),
        "OPENCODE_CONFIG_DIR=/opencode-config-dir".to_string(),
    ]);
}

// OPENCODE_CONFIG special case: bind-mount file with container path rewrite.
let opencode_config_env = resolve_config_file_env(
    env.get("OPENCODE_CONFIG").cloned(),
    opts.host_home_dir.as_deref(),
)?;
if let Some(config_file_env) = &opencode_config_env {
    args.extend([
        "-v".to_string(),
        format!("{}:/opencode.json:ro", config_file_env.display()),
        "-e".to_string(),
        "OPENCODE_CONFIG=/opencode.json".to_string(),
    ]);
}
```

### Environment Passthrough
File: `src/dev/opencode/env.rs`
```rust
pub const PASSTHROUGH_VARS: &[&str] = &[
    // ...
    // Configuration with Paths (users must provide container paths)
    "OPENCODE_CONFIG",
    "OPENCODE_CONFIG_CONTENT",
    "OPENCODE_MODELS_PATH",
];
```
*Note: `OPENCODE_CONFIG` is currently in `PASSTHROUGH_VARS`, which would normally pass the host value. The special handling in `mod.rs` overrides this by setting it to `/opencode.json`.*

## Feasibility of Replacement

The `extra_data_volumes` feature provides a robust alternative for setting up these mounts.

### 1. Volume Mounting
The `extra_data_volumes` mechanism in `src/dev/volumes.rs` already supports:
- Bind mounts (`type: "bind"`)
- Tilde expansion in host sources (`~/...`)
- Mode selection (`ro`, `rw`)

Users can achieve the same mounting behavior by adding entries to `cast.json`:
```json
"extra_data_volumes": {
  "opencode_config": {
    "target": "/opencode.json",
    "source": "~/.config/opencode/config.json",
    "type": "bind",
    "mode": "ro"
  }
}
```

### 2. Environment Variables
The missing piece in `cast.json` is a direct way to set environment variables for the main container. However, this is already addressed by `cast.env` support in `src/dev/env_file.rs`, which automatically loads `./cast.env` or `~/.config/cast/cast.env`.

Users can set the required environment variables there:
```
OPENCODE_CONFIG=/opencode.json
```

## Benefits of Dropping Special Handling

1. **Logic Simplification**: Removes approximately 25 lines of specialized logic from `src/dev/opencode/mod.rs` and the supporting `resolve_config_dir_env`/`resolve_config_file_env` functions.
2. **Transparency**: All volume mounts and environment overrides become explicit in the configuration files (`cast.json`, `cast.env`) rather than being hidden in agent-specific code.
3. **Consistency**: Aligns the `OpenCode` agent with other agents and leverages the core `cast` features instead of creating special-case silos.

## Findings Summary

- **Can it be dropped?** Yes.
- **Is there a regression in functionality?** No, though it requires users to be more explicit in their configuration.
- **Is additional code needed?** No new code is required to support the replacement, as the necessary features (`extra_data_volumes` and `cast.env`) already exist.
