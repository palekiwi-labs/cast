use std::io::Write;
use std::process::ExitCode;

use crate::config::{
    ApprovalStatus, Config, ConfigDiffOutput, compute_workspace_diff, get_approval_status,
};
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
    let user = get_user()?;
    let workspace = get_workspace(&user.username)?;

    match command {
        Some(ConfigCommands::Show) | None => {
            println!("{}", serde_json::to_string_pretty(config)?);

            let hint = match get_approval_status(config, &workspace.root)? {
                ApprovalStatus::Approved => None,
                ApprovalStatus::Changed => Some(
                    "Note: config changed since last approval — run `cast config diff` to see what changed, or `cast config allow` to approve.",
                ),
                ApprovalStatus::Unapproved => Some(
                    "Note: config not yet approved — run `cast config allow` to approve the current configuration.",
                ),
            };
            if let Some(msg) = hint {
                writeln!(std::io::stderr().lock(), "{}", msg)?;
            }

            Ok(ExitCode::SUCCESS)
        }
        Some(ConfigCommands::Allow) => {
            crate::config::approve_workspace_config(config, &workspace.root)?;
            Ok(ExitCode::SUCCESS)
        }
        Some(ConfigCommands::Deny) => {
            crate::config::deny_workspace_config(&workspace.root)?;
            Ok(ExitCode::SUCCESS)
        }
        Some(ConfigCommands::Diff) => {
            match compute_workspace_diff(config, &workspace.root)? {
                ConfigDiffOutput::Unapproved => {
                    println!(
                        "No approved config for this workspace.\nRun `cast config allow` to approve the current configuration."
                    );
                }
                ConfigDiffOutput::Unchanged => {
                    println!("Config matches approved state. No changes.");
                }
                ConfigDiffOutput::Changed(diff) => {
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

            Ok(ExitCode::SUCCESS)
        }
    }
}
