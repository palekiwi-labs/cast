use crate::config::Config;
use crate::dev::agent::Agent;
use crate::dev::port::resolve_port;
use anyhow::Result;

pub fn handle_port(config: &Config, agent: &dyn Agent) -> Result<()> {
    let port = resolve_port(config, agent.name())?;
    println!("{}", port);
    Ok(())
}
