use anyhow::Result;

use crate::config::Config;
use crate::dev;
use crate::dev::opencode::OpenCodeHarness;

pub fn handle_opencode(config: &Config, extra_args: Vec<String>) -> Result<()> {
    dev::run_harness(&OpenCodeHarness, config, extra_args)
}
