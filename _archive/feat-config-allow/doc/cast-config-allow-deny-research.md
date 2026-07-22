# Research Report: `cast config allow/deny`

This report documents the codebase research for implementing configuration hash-based approval in `cast`.

## Research Questions Answered
1. How is `cast` configuration assembled and what fields are sensitive?
2. Where and how should approved hashes be persisted?
3. Where is the optimal interception point in the `cast run` execution flow?
4. How should the new subcommands be integrated into the CLI?

## 1. Configuration Analysis & Hashing

### Sensitive Fields
The `Config` struct in `/home/pl/code/palekiwi-labs/cast/src/config/schema.rs` defines the sandbox boundaries. The following fields are critical for security and must be included in any deterministic hash:

- `memory`, `cpus`, `pids_limit`: Resource constraints.
- `network`: Connectivity level.
- `extra_data_volumes`: Host-to-container bind mounts.
- `forbidden_paths`: Path shadowing definitions.
- `nix_extra_substituters`, `nix_extra_trusted_public_keys`: Binary trust roots.
- `use_flake`, `use_flake_path`: Nix environment source.

### Deterministic Representation
The `Config` struct implements `serde::Serialize`. A deterministic hash can be produced by serializing to canonical JSON (sorted keys) using `serde_json`.

**Source:** `/home/pl/code/palekiwi-labs/cast/src/config/schema.rs`
```rust
#[derive(Clone, Debug, Deserialize, Serialize, Figment)]
pub struct Config {
    #[serde(default = "default_memory")]
    pub memory: String,
    // ...
    #[serde(default)]
    pub extra_data_volumes: Vec<Volume>,
    // ...
}
```

## 2. Persistence of Approvals

### Storage Location
Existing state (like agent versions) is stored in `~/.cache/cast/versions/` using `dirs::cache_dir()`. For approved hashes, which are more permanent than a cache, `dirs::data_dir()` (`~/.local/share/cast/`) is the appropriate location following XDG standards.

**Reference Pattern:** `/home/pl/code/palekiwi-labs/cast/src/dev/version/cache.rs`
```rust
pub fn get_cache_path(agent_name: &str) -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from(".cache"))
        .join("cast")
        .join("versions")
        .join(format!("{}-version-cache.json", agent_name))
}
```

### Proposed Schema
A JSON mapping of hashes to metadata:
```json
{
  "hashes": {
    "<sha256-hash>": {
      "workspace": "/absolute/path/to/project",
      "approved_at": 1715234400
    }
  }
}
```

## 3. Execution Interception

### The `run_agent` Flow
The central orchestration point is `run_agent` in `src/dev/run.rs`. The config is already loaded and passed as an argument. The check should occur after resolving the workspace path but before any side-effecting operations (like starting the Nix daemon or pulling images).

**Source:** `/home/pl/code/palekiwi-labs/cast/src/dev/run.rs`
```rust
pub fn run_agent(
    agent: &dyn Agent,
    config: &Config,
    extra_args: Vec<String>,
) -> Result<ExitStatus> {
    let start_time = Instant::now();
    let docker = DockerClient;
    let user = get_user()?;
    let workspace = get_workspace(&user.username)?;

    // PROPOSED INTERCEPTION POINT HERE
    
    nix_daemon::ensure_running(&docker, config)?;
    // ...
}
```

## 4. CLI Integration

`cast` uses `clap` with a derived subcommands enum. `config allow` and `config deny` should be added to the `ConfigCommands` enum.

**Source:** `/home/pl/code/palekiwi-labs/cast/src/commands/config.rs`
```rust
#[derive(Debug, Subcommand)]
pub enum ConfigCommands {
    /// Show the current configuration
    Show,
}

pub fn handle_config(command: ConfigCommands, config: &Config) -> Result<()> {
    match command {
        ConfigCommands::Show => {
            println!("{}", serde_json::to_string_pretty(config)?);
            Ok(())
        }
    }
}
```

## Summary of Findings
- **Hashing**: Use `serde_json` to bytes + `sha2` (SHA-256). Include the absolute workspace path to bind approvals to specific projects.
- **Persistence**: Store in `~/.local/share/cast/approved_configs.json`.
- **Interception**: Early in `src/dev/run.rs#run_agent`.
- **Exit**: Return a non-zero `ExitStatus` to `cli.rs` when approval is missing.
