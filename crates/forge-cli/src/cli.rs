//! CLI argument parsing: top-level `Cli`/`Commands` and subcommand enums.

use clap::{Parser, Subcommand};

use crate::commands::hook::HookAddArgs;
use crate::commands::migrate::MigrateArgs;
use crate::commands::register::RegisterArgs;
use crate::commands::token::TokenMintArgs;

#[derive(Parser)]
#[command(name = "forge", about = "Flint Forge operator CLI")]
#[command(version)]
pub struct Cli {
    /// Run the command inside the flint-forge-cli container.
    #[arg(long, env = "FORGE_CONTAINER")]
    pub container: bool,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Print CLI and workspace version.
    Version,
    /// Manage Kiln functions.
    #[command(name = "fn")]
    Function {
        #[command(subcommand)]
        command: FunctionCommands,
    },
    /// Manage webhook dispatch rules.
    #[command(name = "hook")]
    Hook {
        #[command(subcommand)]
        command: HookCommands,
    },
    /// Apply SQL migrations.
    Migrate(MigrateArgs),
    /// Manage operator tokens.
    #[command(name = "token")]
    Token {
        #[command(subcommand)]
        command: TokenCommands,
    },
}

#[derive(Subcommand)]
pub enum FunctionCommands {
    /// Register a WASM component with the Kiln control-plane.
    Register(RegisterArgs),
}

#[derive(Subcommand)]
pub enum HookCommands {
    /// Add a webhook dispatch rule for a table.
    Add(HookAddArgs),
}

#[derive(Subcommand)]
pub enum TokenCommands {
    /// Mint a smoke-test JWT.
    Mint(TokenMintArgs),
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::Parser;

    use super::*;

    #[test]
    fn parse_version_command() {
        let cli = Cli::try_parse_from(["forge", "version"]).unwrap();
        assert!(matches!(cli.command, Commands::Version));
        assert!(!cli.container);
    }

    #[test]
    fn parse_fn_register_defaults() {
        let cli = Cli::try_parse_from(["forge", "fn", "register", "foo.wasm"]).unwrap();
        match cli.command {
            Commands::Function {
                command: FunctionCommands::Register(args),
            } => {
                assert_eq!(args.path, PathBuf::from("foo.wasm"));
                assert_eq!(args.version, "1.0.0");
                assert_eq!(args.admin_url, "http://localhost:8090");
            }
            _ => panic!("expected fn register"),
        }
    }

    #[test]
    fn parse_hook_add_defaults() {
        let cli = Cli::try_parse_from([
            "forge",
            "hook",
            "add",
            "public.tasks",
            "https://example.com",
        ])
        .unwrap();
        match cli.command {
            Commands::Hook {
                command: HookCommands::Add(args),
            } => {
                assert_eq!(args.table, "public.tasks");
                assert_eq!(args.url, "https://example.com");
                assert_eq!(args.events, vec!["INSERT", "UPDATE", "DELETE"]);
                assert_eq!(args.tier, "standard");
            }
            _ => panic!("expected hook add"),
        }
    }

    #[test]
    fn parse_migrate_defaults() {
        let cli = Cli::try_parse_from(["forge", "migrate"]).unwrap();
        match cli.command {
            Commands::Migrate(args) => {
                assert_eq!(args.source, PathBuf::from("migrations"));
            }
            _ => panic!("expected migrate"),
        }
    }

    #[test]
    fn parse_token_mint_defaults() {
        let cli = Cli::try_parse_from(["forge", "token", "mint", "--secret", "x"]).unwrap();
        match cli.command {
            Commands::Token {
                command: TokenCommands::Mint(args),
            } => {
                assert_eq!(args.secret, Some("x".to_owned()));
                assert_eq!(args.subject, "smoke");
                assert_eq!(args.expiry_seconds, 3600);
            }
            _ => panic!("expected token mint"),
        }
    }
}
