use anyhow::Result;
use std::env;
use std::io::Write;
use std::process::{Command, Stdio};

use crate::config::load_config;

/// Calculate a deterministic port based on the current working directory
pub fn calculate_port() -> Result<u16> {
    let pwd = env::current_dir()?;
    let pwd_str = pwd.to_string_lossy();

    // Use cksum to generate a hash of the directory path
    let mut child = Command::new("cksum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    // Write the path to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(pwd_str.as_bytes())?;
    }

    let output = child.wait_with_output()?;

    if !output.status.success() {
        anyhow::bail!("Failed to run cksum command");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse the checksum value (first field in output)
    let checksum: u32 = stdout
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("Failed to parse cksum output"))?;

    // Map to ephemeral port range: 32768-65535
    let port = 32768 + (checksum % 32768);

    Ok(port as u16)
}

/// Get the port that will be published by the container
/// Returns the configured port if set, otherwise calculates a deterministic port
pub fn get_port() -> Result<u16> {
    let config = load_config()?;

    match config.port {
        Some(port) => Ok(port),
        None => calculate_port(),
    }
}

pub fn handle_port() -> Result<()> {
    let port = get_port()?;
    println!("{}", port);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_port_in_range() {
        let port = calculate_port().expect("Should calculate port");
        assert!(
            port >= 32768,
            "Port should be in ephemeral range (>= 32768)"
        );
    }

    #[test]
    fn test_calculate_port_deterministic() {
        // Same directory should always give same port
        let port1 = calculate_port().expect("Should calculate port");
        let port2 = calculate_port().expect("Should calculate port");
        assert_eq!(port1, port2, "Port should be deterministic");
    }
}
