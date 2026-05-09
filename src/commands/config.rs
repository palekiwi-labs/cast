use std::process::ExitCode;

use crate::config::{compute_config_hash, load_approval_store, Config};
use crate::dev::workspace::get_workspace;
use crate::user::get_user;
use anyhow::Result;

#[derive(clap::Subcommand)]
pub enum ConfigCommands {
    /// Show the current configuration
    Show,
    /// Approve the current configuration for this project
    Allow,
    /// Revoke approval for the current configuration in this project
    Deny,
}

pub fn handle_config(config: &Config, command: Option<ConfigCommands>) -> Result<ExitCode> {
    match command {
        Some(ConfigCommands::Show) | None => {
            let json = serde_json::to_string_pretty(&config)?;
            println!("{}", json);
            Ok(ExitCode::SUCCESS)
        }
        Some(ConfigCommands::Allow) => {
            let user = get_user()?;
            let workspace = get_workspace(&user.username)?;
            let hash = compute_config_hash(config, &workspace.root)?;
            let mut store = load_approval_store()?;
            store.add_entry(hash.clone(), workspace.root.to_string_lossy().into_owned());
            store.save()?;
            println!("Configuration approved ({}).", &hash[..12]);
            Ok(ExitCode::SUCCESS)
        }
        Some(ConfigCommands::Deny) => {
            let user = get_user()?;
            let workspace = get_workspace(&user.username)?;
            let hash = compute_config_hash(config, &workspace.root)?;
            let mut store = load_approval_store()?;
            store.remove_entry(&hash);
            store.save()?;
            println!("Approval revoked ({}).", &hash[..12]);
            Ok(ExitCode::SUCCESS)
        }
    }
}
