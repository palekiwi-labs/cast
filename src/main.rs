use cast::commands::{run, Cli};
use clap::Parser;

fn main() {
    if let Err(e) = run_cli() {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}

fn run_cli() -> anyhow::Result<()> {
    let cli = Cli::parse();
    run(cli)
}
