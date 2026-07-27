//! Per-run artifact directory: the single addressable handle bundling the
//! prompt, the live-flushed trace, child stderr, cast-agent's PID, and the
//! structured verdict.
//!
//! Base-dir precedence: `--run-dir` flag > `CAST_AGENT_RUN_DIR` env >
//! `${TMPDIR:-/tmp}/cast-agent/runs/`. The default unifies test and prod paths
//! (tests use `std::env::temp_dir()` == `TMPDIR`) and is Nix-sandbox safe.

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Resolve the base directory under which per-run subdirs are created.
pub fn resolve_base(flag: Option<PathBuf>, env: Option<String>, tmpdir: Option<String>) -> PathBuf {
    if let Some(f) = flag {
        return f;
    }
    if let Some(e) = env {
        return PathBuf::from(e);
    }
    let root = tmpdir.unwrap_or_else(|| "/tmp".to_string());
    PathBuf::from(root).join("cast-agent").join("runs")
}

/// Resolve the base from the live process environment.
pub fn resolve_base_from_env(flag: Option<PathBuf>) -> PathBuf {
    resolve_base(
        flag,
        std::env::var("CAST_AGENT_RUN_DIR").ok(),
        std::env::var("TMPDIR").ok(),
    )
}

/// Create a unique per-run subdirectory under `base`.
///
/// The name is timestamp-first (sortable) and embeds the harness and PID plus
/// a sub-second nanosecond component so that runs sharing a PID (recycled) or
/// launched within the same second cannot collide.
pub fn create_run_dir(base: &Path, harness: &str) -> Result<PathBuf> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let pid = std::process::id();
    // `<secs><nanos:09>` is a zero-padded, lexically-sortable numeric prefix.
    let name = format!(
        "{}{:09}-{}-{}",
        now.as_secs(),
        now.subsec_nanos(),
        harness,
        pid
    );
    let dir = base.join(name);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
