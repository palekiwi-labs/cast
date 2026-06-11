use cast_mcp_client::commands::{Cli, run};
use cast_mcp_client::print_json_error;
use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(err) = run(cli).await {
        print_json_error("COMMAND_ERROR", &err.to_string());
        std::process::exit(1);
    }
}
