mod cmd;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "llm-cascade",
    about = "Cascading LLM inference across multiple providers"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run a cascade with a prompt or conversation file
    Run(cmd::run::RunArgs),
    /// Initialize configuration and directories
    Setup(cmd::setup::SetupArgs),
    /// Manage API keys in the OS keyring
    Key(cmd::key::KeyArgs),
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Run(args) => cmd::run::execute(args),
        Command::Setup(args) => cmd::setup::execute(args),
        Command::Key(args) => cmd::key::execute(args),
    }
}
