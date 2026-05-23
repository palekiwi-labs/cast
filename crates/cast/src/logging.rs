use anyhow::{Context, Result};
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use tracing::Level;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;

pub fn generate_invocation_id() -> String {
    let s = RandomState::new();
    let hasher = s.build_hasher();
    let hash = hasher.finish();
    format!("{:08x}", hash as u32)
}

pub fn init_file_logger() -> Result<()> {
    let log_dir = std::env::var_os("CAST_LOG_DIR")
        .map(std::path::PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("fallback"))
        .or_else(|_| {
            dirs::data_dir()
                .context("Failed to get data directory")
                .map(|d| d.join("cast/logs"))
        })?;

    std::fs::create_dir_all(&log_dir).context("Failed to create log directory")?;

    let file_appender = tracing_appender::rolling::daily(log_dir, "cast");

    let env_filter = EnvFilter::try_from_env("CAST_LOG")
        .unwrap_or_else(|_| EnvFilter::new(Level::INFO.to_string()));

    tracing_subscriber::fmt()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_env_filter(env_filter)
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
        .init();

    Ok(())
}
