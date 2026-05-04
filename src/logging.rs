use anyhow::{Context, Result};
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;

pub fn generate_invocation_id() -> String {
    let s = RandomState::new();
    let mut hasher = s.build_hasher();
    hasher.write_u64(0); // Add some entropy if needed, but build_hasher is already seeded
    let hash = hasher.finish();
    format!("{:08x}", hash as u32)
}

pub fn init_file_logger() -> Result<()> {
    let log_dir = dirs::data_dir()
        .context("Failed to get data directory")?
        .join("cast/logs");

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
