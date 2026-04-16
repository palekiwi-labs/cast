use anyhow::Result;

#[derive(clap::Subcommand)]
pub enum ConfigCommands {
    /// Show the current configuration
    Show,
}

pub fn handle_config(command: Option<ConfigCommands>) -> Result<()> {
    match command {
        Some(ConfigCommands::Show) | None => {
            let config = crate::config::load_config()?;
            let json = serde_json::to_string_pretty(&config)?;
            println!("{}", json);
            Ok(())
        }
    }
}
