pub mod cli;
mod config;
#[cfg(feature = "mcp")]
pub mod mcp;
mod nix_daemon;
mod tui;

pub use cli::{Cli, run};
