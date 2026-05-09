use std::process::ExitCode;

use crate::config::Config;
use anyhow::Result;

#[derive(clap::Subcommand)]
pub enum ConfigCommands {
    /// Show the current configuration
    Show,
}

pub fn handle_config(config: &Config, command: Option<ConfigCommands>) -> Result<ExitCode> {
    match command {
        Some(ConfigCommands::Show) | None => {
            let json = serde_json::to_string_pretty(&config)?;
            println!("{}", json);
            Ok(ExitCode::SUCCESS)
        }
    }
}
