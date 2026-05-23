use std::io::Write;
use std::process::ExitCode;

use crate::config::{Config, format_config_diff, load_approval_store};
use crate::dev::workspace::get_workspace;
use crate::user::get_user;
use anyhow::Result;
use owo_colors::OwoColorize;

#[derive(clap::Subcommand)]
pub enum ConfigCommands {
    /// Show the current configuration
    Show,
    /// Approve the current configuration for this project
    Allow,
    /// Revoke approval for the current configuration in this project
    Deny,
    /// Show a diff between the last approved config and the current state
    Diff,
}

pub fn handle_config(config: &Config, command: Option<ConfigCommands>) -> Result<ExitCode> {
    match command {
        Some(ConfigCommands::Show) | None => {
            let json = serde_json::to_string_pretty(&config)?;
            println!("{}", json);

            // Emit a hint to stderr if the config is not yet approved
            let user = get_user()?;
            let workspace = get_workspace(&user.username)?;
            let store = load_approval_store()?;
            let hash = crate::config::compute_config_hash(config, &workspace.root)?;
            if !store.is_approved(&hash) {
                let stderr = std::io::stderr();
                writeln!(
                    &mut stderr.lock(),
                    "Note: config not approved — run `cast config diff` to see what changed, or `cast config allow` to approve."
                )?;
            }

            Ok(ExitCode::SUCCESS)
        }
        Some(ConfigCommands::Allow) => {
            let user = get_user()?;
            let workspace = get_workspace(&user.username)?;
            crate::config::approve_workspace_config(config, &workspace.root)?;
            Ok(ExitCode::SUCCESS)
        }
        Some(ConfigCommands::Deny) => {
            let user = get_user()?;
            let workspace = get_workspace(&user.username)?;
            crate::config::deny_workspace_config(&workspace.root)?;
            Ok(ExitCode::SUCCESS)
        }
        Some(ConfigCommands::Diff) => {
            let user = get_user()?;
            let workspace = get_workspace(&user.username)?;
            let canonical = std::fs::canonicalize(&workspace.root)?;
            let canonical_str = canonical.to_string_lossy();

            let store = load_approval_store()?;

            match store.find_by_workspace(&canonical_str) {
                None => {
                    println!(
                        "No approved config for this workspace.\nRun `cast config allow` to approve the current configuration."
                    );
                }
                Some(entry) if entry.approved_config.is_none() => {
                    println!(
                        "No config snapshot available for this workspace.\n\
                         This entry was approved with an older version of cast.\n\
                         Run `cast config allow` to re-approve and capture a snapshot."
                    );
                }
                Some(entry) => {
                    let approved_snapshot = entry.approved_config.as_ref().unwrap();
                    let current_hash = crate::config::compute_config_hash(config, &canonical)?;

                    // Find the hash key for this entry by looking it up
                    let is_unchanged = store
                        .entries
                        .get(&current_hash)
                        .map(|e| e.workspace == canonical_str)
                        .unwrap_or(false);

                    if is_unchanged {
                        println!("Config matches approved state. No changes.");
                    } else {
                        let current_value = serde_json::to_value(config)?;
                        let diff = format_config_diff(approved_snapshot, &current_value);

                        if diff.is_empty() {
                            println!("Config matches approved state. No changes.");
                        } else {
                            let use_color = std::io::IsTerminal::is_terminal(&std::io::stdout());
                            for line in diff.lines() {
                                if use_color {
                                    if let Some(rest) = line.strip_prefix('+') {
                                        println!("{}", format!("+{}", rest).green());
                                    } else if let Some(rest) = line.strip_prefix('-') {
                                        println!("{}", format!("-{}", rest).red());
                                    } else {
                                        println!("{}", line.dimmed());
                                    }
                                } else {
                                    println!("{}", line);
                                }
                            }
                        }
                    }
                }
            }

            Ok(ExitCode::SUCCESS)
        }
    }
}
