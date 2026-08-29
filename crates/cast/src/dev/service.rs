use crate::config::Config;

pub fn build_service_command(config: &Config, username: &str) -> Vec<String> {
    let shell = config.global_shell.as_deref().unwrap_or("universal");

    vec![
        "nix".to_string(),
        "develop".to_string(),
        format!("/home/{username}/.config/cast/nix#{shell}"),
        "-c".to_string(),
        "herdr".to_string(),
        "server".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_command_starts_herdr_in_the_configured_global_shell() {
        let config = Config {
            global_shell: Some("project".to_string()),
            ..Default::default()
        };

        let command = build_service_command(&config, "alice");

        assert_eq!(
            command,
            vec![
                "nix",
                "develop",
                "/home/alice/.config/cast/nix#project",
                "-c",
                "herdr",
                "server",
            ]
        );
    }
}
