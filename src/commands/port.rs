use crate::config::Config;
use crate::dev::port::resolve_port;
use anyhow::Result;

pub fn handle_port(config: &Config) -> Result<()> {
    let port = resolve_port(config)?;
    println!("{}", port);
    Ok(())
}
