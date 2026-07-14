mod cli;
mod commands;
mod container;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Commands, FunctionCommands, HookCommands, TokenCommands};
use container::INSIDE_CONTAINER_ENV;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    if cli.container && std::env::var(INSIDE_CONTAINER_ENV).is_err() {
        return container::run_in_container(&cli).await;
    }

    match cli.command {
        Commands::Version => {
            print_version();
            Ok(())
        }
        Commands::Function {
            command: FunctionCommands::Register(args),
        } => commands::register::register(args).await,
        Commands::Hook {
            command: HookCommands::Add(args),
        } => commands::hook::hook_add(args).await,
        Commands::Migrate(args) => commands::migrate::migrate(args).await,
        Commands::Token {
            command: TokenCommands::Mint(args),
        } => commands::token::token_mint(args),
    }
}

fn print_version() {
    println!("{}", env!("CARGO_PKG_VERSION"));
}
