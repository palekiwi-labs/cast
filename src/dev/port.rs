use anyhow::Result;
use crc::{Crc, CRC_32_CKSUM};
use std::env;

use crate::config::Config;

/// The POSIX cksum algorithm implementation
const CKSUM: Crc<u32> = Crc::<u32>::new(&CRC_32_CKSUM);

/// Calculate a deterministic port based on the current working directory
pub fn calculate_port() -> Result<u16> {
    let pwd = env::current_dir()?;
    let pwd_str = pwd.to_string_lossy();

    // The POSIX cksum algorithm includes the length of the string
    // appended as little-endian bytes to the end of the digest calculation
    let mut digest = CKSUM.digest();
    digest.update(pwd_str.as_bytes());

    // cksum requires appending the length in a specific way
    let mut len = pwd_str.len();
    while len > 0 {
        digest.update(&[(len & 0xFF) as u8]);
        len >>= 8;
    }

    let checksum = digest.finalize();

    // Map to ephemeral port range: 32768-65535
    let port = 32768 + (checksum % 32768);

    Ok(port as u16)
}

/// Resolve the port to use, either from config or by calculating it
pub fn resolve_port(config: &Config) -> Result<u16> {
    match config.port {
        Some(port) => Ok(port),
        None => calculate_port(),
    }
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

    #[test]
    fn test_resolve_port_with_config_override() {
        let mut config = Config::default();
        config.port = Some(12345);
        assert_eq!(resolve_port(&config).unwrap(), 12345);
    }

    #[test]
    fn test_resolve_port_without_config_uses_calculated() {
        let config = Config::default();
        let calculated = calculate_port().unwrap();
        assert_eq!(resolve_port(&config).unwrap(), calculated);
    }

    #[test]
    fn test_cksum_algorithm_matches_posix() {
        // Test our native CRC implementation against known cksum outputs
        let test_cases = vec![
            ("test", 3076352578),
            ("hello world", 1135714720),
            ("ocx-rs is awesome", 1458670426),
        ];

        for (input, expected_checksum) in test_cases {
            let mut digest = CKSUM.digest();
            digest.update(input.as_bytes());

            let mut len = input.len();
            while len > 0 {
                digest.update(&[(len & 0xFF) as u8]);
                len >>= 8;
            }

            let checksum = digest.finalize();
            assert_eq!(
                checksum, expected_checksum,
                "Checksum for '{}' did not match. Expected {}, got {}",
                input, expected_checksum, checksum
            );
        }
    }
}
