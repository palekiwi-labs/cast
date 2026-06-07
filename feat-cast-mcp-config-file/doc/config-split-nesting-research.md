# Research: Removing Root Key Requirement in `cast-mcp.json`

This report investigates how to load `cast-mcp.json` such that it does not require a root `mcp` key, while still merging its contents into the `mcp` field of the main `Config` struct.

## Research Questions Answered

### 1. Does `figment` support merging a file under a specific key (nesting)?
`figment` does not provide a direct wrapper to nest an entire `Provider` (like `Json::file`) under a key. However, it provides a standard pattern for this: extract the data into an intermediate `figment::value::Value` and then merge that value using a keyed provider.

### 2. What is the recommended implementation?
To allow `cast-mcp.json` to have a flat structure (no root `mcp` key), the loading logic in `crates/cast/src/config/loader.rs` should be modified as follows:

```rust
// 1. Load cast-mcp.json into an intermediate figment::value::Value
let mcp_json: figment::value::Value = Figment::from(Json::file("cast-mcp.json"))
    .extract()
    .unwrap_or_else(|_| figment::value::Value::Dict(figment::value::Profile::Default, figment::value::Dict::new()));

// 2. Merge it into the main figment under the "mcp" key
let config: Config = figment
    .merge(Json::file("cast.json"))
    .merge(figment::providers::Serialized::defaults(mcp_json).key("mcp"))
    .merge(Env::prefixed("CAST_").split("__"))
    .extract()?;
```

### 3. How does this handle missing files?
`Figment::from(Json::file("cast-mcp.json"))` will treat a missing file as an empty dictionary. The `.extract::<Value>()` call will succeed and return an empty `Value::Dict`. Merging this empty dict under the `mcp` key will have no effect on existing `mcp` settings (from `cast.json` or defaults), which is the desired behavior.

## Sourced Findings

### `figment` Nesting Capabilities
- **`figment::providers::Data::nested()`**: Used for profile-based nesting (e.g., `[debug]`, `[release]`), not for namespacing keys.
- **`figment::providers::Serialized::key(path)`**: The primary mechanism for namespacing data. It requires a serializable value.
- **`figment::value::Value`**: Implements `Serialize`, making it the perfect bridge between a `Provider` and a `Serialized` wrapper.

**Source**: `figment` source code analysis (`src/providers/data.rs`, `src/providers/serialized.rs`, `src/figment.rs`).

## Implementation Note
As the **Research Coordinator**, I provide these findings for the purpose of technical guidance. Implementation should be performed by an agent with the appropriate permissions and role.
