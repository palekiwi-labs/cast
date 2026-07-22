# Research: Splitting `cast` Configuration into `cast.json` and `cast-mcp.json`

This report investigates the feasibility of splitting the configuration for the `cast` crate into two files: `cast.json` (general settings) and `cast-mcp.json` (MCP-specific settings).

## Research Questions Answered

### 1. How does `cast` currently manage its configuration?
The configuration is managed using the `figment` crate. It follows a specific precedence order where later sources override earlier ones:
1. **Defaults**: Hardcoded in `src/config/schema.rs`.
2. **Global Config**: `~/.config/cast/cast.json`.
3. **Local Config**: `./cast.json` in the current working directory.
4. **Environment Variables**: Variables prefixed with `CAST_` (e.g., `CAST_MEMORY`).

**Source**: `crates/cast/src/config/loader.rs`
**Symbol**: `load_config`
```rust
pub fn load_config() -> Result<Config> {
    let mut figment = Figment::new().merge(Serialized::defaults(Config::default()));

    if let Some(global_path) = global_config_path() {
        figment = figment.merge(Json::file(global_path));
    }

    let config: Config = figment
        .merge(Json::file("cast.json"))
        .merge(Env::prefixed("CAST_").split("__"))
        .extract()
        .context("Failed to load configuration")?;
    // ...
}
```

### 2. Does `figment` support loading and merging multiple files?
Yes. `figment` is designed to combine multiple providers (JSON files, environment variables, etc.). You can chain multiple `.merge()` calls to include additional configuration files.

**Source**: `figment` documentation and source (`src/figment.rs`).

### 3. How does `figment` handle merging nested structures?
When multiple sources contain the same key pointing to a dictionary (like the `mcp` field), `figment` performs a **recursive union**. It does not simply overwrite the entire nested object; instead, it merges their individual fields.

For example, if `cast.json` has general MCP settings and `cast-mcp.json` has specific tool definitions, both will be merged into the final `Config.mcp` object.

**Source**: `figment` source (`src/coalesce.rs`).

### 4. How does `figment` handle missing files?
`figment` natively handles missing files by treating them as empty configuration sources. If `cast-mcp.json` does not exist, it will not error but will simply emit an empty dictionary during the merge process.

**Source**: `figment` source (`src/providers/data.rs`).

### 5. What are the security implications for configuration approval?
`cast` uses an "approval" system where the user must run `cast config allow` to authorize a specific configuration. This is based on a SHA-256 hash of the fully resolved `Config` struct.

Because the proposed change involves merging `cast-mcp.json` into the same `Config` struct before hashing, **the security model remains intact**. Any changes to `cast-mcp.json` will result in a different hash, requiring the user to re-approve the configuration.

**Source**: `crates/cast/src/config/approval.rs`
**Symbol**: `compute_config_hash`
```rust
pub fn compute_config_hash(config: &Config, workspace_root: &Path) -> Result<String> {
    let config_bytes = serde_json::to_vec(config)?; // Serializes the resolved Config object
    // ... hashing logic ...
}
```

## Sourced Findings

### Current Config Structure
The `mcp` configuration is a field within the main `Config` struct:
**Source**: `crates/cast/src/config/schema.rs`
```rust
pub struct Config {
    // ...
    #[serde(default)]
    pub mcp: McpConfig,
}

pub struct McpConfig {
    pub port: u16,
    pub hostname: String,
    pub tools: BTreeMap<String, McpToolConfig>,
}
```

### Proposed Loading Implementation
The loading chain in `crates/cast/src/config/loader.rs` can be updated as follows:
```rust
let config: Config = figment
    .merge(Json::file("cast.json"))
    .merge(Json::file("cast-mcp.json")) // Add this
    .merge(Env::prefixed("CAST_").split("__"))
    .extract()?;
```

## Confidence Notes
- **High**: The `figment` crate's behavior is well-documented and confirmed via source code analysis.
- **High**: The impact on the security model was verified by examining the `compute_config_hash` implementation.
- **High**: The recursive merging behavior ensures that the split can be partial (e.g., some MCP settings in `cast.json` and some in `cast-mcp.json`).
