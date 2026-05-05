use cast::commands::{run, Cli};
use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run_cli() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {:#}", e);
            ExitCode::FAILURE
        }
    }
}

fn run_cli() -> anyhow::Result<ExitCode> {
    let cli = Cli::parse();
    run(cli)
}
