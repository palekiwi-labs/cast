use anyhow::Result;

use crate::config::Config;
use crate::dev;
use crate::dev::opencode::OpenCode;

pub fn handle_opencode(config: &Config, extra_args: Vec<String>) -> Result<()> {
    dev::run_agent(&OpenCode, config, extra_args)
}
