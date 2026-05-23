pub mod cli;
mod config;
#[cfg(feature = "mcp")]
pub mod mcp;
mod nix_daemon;
mod port;

pub use cli::{Cli, run};
