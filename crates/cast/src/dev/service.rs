use crate::config::Config;
use crate::dev::build_command::build_sandbox_command;

pub fn build_service_command(config: &Config, username: &str) -> Vec<String> {
    build_sandbox_command(config, username, "herdr", vec!["server".to_string()])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_command_starts_herdr_in_the_sandbox_shell_only() {
        let config = Config {
            sandbox_shell: Some("~/.config/cast/nix#default".to_string()),
            project_shell: Some(".#project".to_string()),
            ..Default::default()
        };

        let command = build_service_command(&config, "alice");

        assert_eq!(
            command,
            vec![
                "nix",
                "develop",
                "/home/alice/.config/cast/nix#default",
                "-c",
                "herdr",
                "server",
            ]
        );
    }

    #[test]
    fn service_command_is_bare_without_a_sandbox_shell() {
        let config = Config {
            project_shell: Some(".#project".to_string()),
            ..Default::default()
        };

        let command = build_service_command(&config, "alice");

        assert_eq!(command, vec!["herdr", "server"]);
    }
}
