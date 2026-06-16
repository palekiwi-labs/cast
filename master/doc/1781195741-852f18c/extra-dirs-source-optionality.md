# Research: Optional `source` field in `VolumeConfig`

## Question
Why is the `source` field optional in `crates/cast/src/dev/extra_dirs.rs` and the underlying `VolumeConfig` struct?

## Findings

The `source` field in `VolumeConfig` is optional because the system computes a default value based on the `volume_type`.

### 1. Bind Mounts
In `crates/cast/src/dev/volumes.rs`, if the type is `bind`, an omitted `source` defaults to the `target` path.

```rust
// crates/cast/src/dev/volumes.rs:56
.unwrap_or_else(|| resolved_target.to_string());
```

### 2. Named Volumes
For named volumes, an omitted `source` defaults to a name generated from the namespace and the configuration key.

```rust
// crates/cast/src/dev/volumes.rs:60
let default_vol_name = format!("{}-{}", cfg.volumes_namespace, key);
let vol_name = vol.source.as_deref().unwrap_or(&default_vol_name);
```

### 3. Extra Dirs Logic
The logic in `crates/cast/src/dev/extra_dirs.rs` specifically resolves directory paths for volume-backed storage. It only requires the `target` path to know where the directory should be mapped or created within the dev environment. It does not use the `source` field at all.

```rust
// crates/cast/src/dev/extra_dirs.rs:12
.filter(|v| v.volume_type == "volume")
.map(|v| {
    let expanded = if let Some(rest) = v.target.strip_prefix("~/") {
        format!("/home/{}/{}", username, rest)
    } else {
        v.target.clone()
    };
    format!("\"{}\"", expanded)
})
```

## Conclusion
The `source` field is optional to provide a better developer experience by defaulting to sensible values (identical path for binds, namespaced IDs for volumes), and is omitted in `extra_dirs.rs` because that module only tracks internal mount targets.
