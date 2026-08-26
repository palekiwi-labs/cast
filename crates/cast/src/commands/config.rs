use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use crate::config::{
    ApprovalStatus, Config, ConfigDiffOutput, compute_workspace_diff, get_approval_status,
};
use crate::dev::workspace::get_workspace;
use crate::user::get_user;
use anyhow::{Context, Result};
use owo_colors::OwoColorize;

const DEFAULT_CAST_JSON: &str = r#"{
  "global_shell": "~/.config/cast/nix#default",
  "nix_extra_substituters": ["https://cache.numtide.com"],
  "nix_extra_trusted_public_keys": [
    "niks3.numtide.com-1:DTx8wZduET09hRmMtKdQDxNNthLQETkc/yaX7M4qK0g="
  ]
}
"#;

const GLOBAL_FLAKE_TEMPLATE: &str =
    include_str!("../../assets/global-flake-template/flake.nix");

#[derive(clap::Subcommand)]
pub enum ConfigCommands {
    /// Create the global cast configuration and Nix flake
    Init,
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
    if matches!(command, Some(ConfigCommands::Init)) {
        init_global_config()?;
        return Ok(ExitCode::SUCCESS);
    }

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
        Some(ConfigCommands::Init) => unreachable!("config init handled before workspace lookup"),
    }
}

fn init_global_config() -> Result<()> {
    let home = dirs::home_dir().context("Failed to resolve user home directory")?;
    let cast_dir = home.join(".config/cast");

    write_if_missing(
        &cast_dir.join("cast.json"),
        DEFAULT_CAST_JSON,
        "global cast config",
    )?;
    write_if_missing(
        &cast_dir.join("nix/flake.nix"),
        GLOBAL_FLAKE_TEMPLATE,
        "global nix flake",
    )?;

    Ok(())
}

fn write_if_missing(path: &Path, contents: &str, description: &str) -> Result<()> {
    if path.exists() {
        eprintln!("Skipped existing {description} at {}", path.display());
        return Ok(());
    }

    let parent = path
        .parent()
        .context("Global config path has no parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating global config directory {}", parent.display()))?;
    fs::write(path, contents)
        .with_context(|| format!("writing {description} {}", path.display()))?;
    eprintln!("Created {description} at {}", path.display());

    Ok(())
}
