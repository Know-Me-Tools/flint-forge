use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use base64::Engine as _;
use chrono::{Duration, Utc};
use clap::{Args, Parser, Subcommand};
use fke_domain::{Capability, FunctionManifest};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::info;
use uuid::Uuid;

const INSIDE_CONTAINER_ENV: &str = "FORGE_CONTAINERIZED";
const CONTAINER_IMAGE: &str = "flint-forge-cli";

#[derive(Parser)]
#[command(name = "forge", about = "Flint Forge operator CLI")]
#[command(version)]
struct Cli {
    /// Run the command inside the flint-forge-cli container.
    #[arg(long, env = "FORGE_CONTAINER")]
    container: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
enum FunctionCommands {
    /// Register a WASM component with the Kiln control-plane.
    Register(RegisterArgs),
}

#[derive(Args)]
struct RegisterArgs {
    /// Path to the .wasm component file.
    path: PathBuf,
    /// Function name. Defaults to the file stem.
    #[arg(short, long)]
    name: Option<String>,
    /// Function version.
    #[arg(short, long, default_value = "1.0.0")]
    version: String,
    /// Publisher DID recorded in the manifest.
    #[arg(long, default_value = "did:flint:operator")]
    publisher_did: String,
    /// Granted capabilities (comma-separated).
    #[arg(long, value_delimiter = ',', default_value = "HttpOutgoing")]
    capabilities: Vec<String>,
    /// Manifest not-before date (RFC3339).
    #[arg(long, default_value_t = default_not_before())]
    not_before: String,
    /// Manifest not-after date (RFC3339).
    #[arg(long, default_value_t = default_not_after())]
    not_after: String,
    /// Kiln control-plane base URL.
    #[arg(long, env = "KILN_ADMIN_URL", default_value = "http://localhost:8090")]
    admin_url: String,
}

#[derive(Subcommand)]
enum HookCommands {
    /// Add a webhook dispatch rule for a table.
    Add(HookAddArgs),
}

#[derive(Args)]
struct HookAddArgs {
    /// Target table as [schema.]table.
    table: String,
    /// Webhook URL.
    url: String,
    /// Events to subscribe to (comma-separated).
    #[arg(short, long, value_delimiter = ',', default_value = "INSERT,UPDATE,DELETE")]
    events: Vec<String>,
    /// Delivery tier: standard or durable.
    #[arg(short, long, default_value = "standard")]
    tier: String,
    /// Database URL.
    #[arg(long, env = "DATABASE_URL", default_value = "postgres://localhost/flint")]
    database_url: String,
    /// Webhook HMAC secret. Generated if omitted.
    #[arg(long)]
    secret: Option<String>,
}

#[derive(Args)]
struct MigrateArgs {
    /// Database URL.
    #[arg(long, env = "DATABASE_URL", default_value = "postgres://localhost/flint")]
    database_url: String,
    /// Migrations directory.
    #[arg(long, default_value = "migrations")]
    source: PathBuf,
}

#[derive(Subcommand)]
enum TokenCommands {
    /// Mint a smoke-test JWT.
    Mint(TokenMintArgs),
}

#[derive(Args)]
struct TokenMintArgs {
    /// JWT signing secret. Falls back to FLINT_JWT_SECRET env.
    #[arg(long, env = "FLINT_JWT_SECRET")]
    secret: Option<String>,
    /// JWT subject.
    #[arg(long, default_value = "smoke")]
    subject: String,
    /// Caller role.
    #[arg(long, default_value = "authenticated")]
    role: String,
    /// Token expiry in seconds.
    #[arg(long, default_value_t = 3600)]
    expiry_seconds: i64,
}

#[derive(Serialize, Deserialize)]
struct RegisterBody {
    name: String,
    version: String,
    manifest: FunctionManifest,
    wasm_base64: String,
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    role: String,
    iat: i64,
    exp: i64,
}

fn default_not_before() -> String {
    Utc::now().to_rfc3339()
}

fn default_not_after() -> String {
    (Utc::now() + Duration::days(365)).to_rfc3339()
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    if cli.container && std::env::var(INSIDE_CONTAINER_ENV).is_err() {
        return run_in_container(&cli).await;
    }

    match cli.command {
        Commands::Version => {
            print_version();
            Ok(())
        }
        Commands::Function {
            command: FunctionCommands::Register(args),
        } => register(args).await,
        Commands::Hook {
            command: HookCommands::Add(args),
        } => hook_add(args).await,
        Commands::Migrate(args) => migrate(args).await,
        Commands::Token {
            command: TokenCommands::Mint(args),
        } => token_mint(args),
    }
}

fn print_version() {
    println!("{}", env!("CARGO_PKG_VERSION"));
}

async fn register(args: RegisterArgs) -> Result<()> {
    let wasm_bytes = tokio::fs::read(&args.path)
        .await
        .with_context(|| format!("failed to read wasm file {}", args.path.display()))?;
    let digest = format!("{:x}", Sha256::digest(&wasm_bytes));
    let wasm_base64 = base64::engine::general_purpose::STANDARD.encode(&wasm_bytes);

    let name = match args.name {
        Some(name) => name,
        None => args
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(str::to_owned)
            .with_context(|| format!("could not derive name from {}", args.path.display()))?,
    };

    let capabilities: Vec<Capability> = args
        .capabilities
        .iter()
        .map(|s| parse_capability(s))
        .collect::<Result<Vec<_>>>()?;

    let manifest = FunctionManifest {
        publisher_did: args.publisher_did,
        content_digest: digest.clone(),
        capabilities,
        version: args.version.clone(),
        not_before: args.not_before,
        not_after: args.not_after,
    };

    let body = RegisterBody {
        name: name.clone(),
        version: args.version,
        manifest,
        wasm_base64,
    };

    let client = reqwest::Client::new();
    let url = format!("{}/admin/functions", args.admin_url.trim_end_matches('/'));
    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .with_context(|| format!("failed to POST to {url}"))?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!("registration failed: {status} {text}");
    }

    info!(name, digest, status = %status, "registered function");
    println!("{text}");
    Ok(())
}

fn parse_capability(value: &str) -> Result<Capability> {
    match value.to_ascii_lowercase().as_str() {
        "db" => Ok(Capability::Db),
        "llm" => Ok(Capability::Llm),
        "kv" => Ok(Capability::Kv),
        "identity" => Ok(Capability::Identity),
        "secrets" => Ok(Capability::Secrets),
        "httpoutgoing" | "http" => Ok(Capability::HttpOutgoing),
        _ => bail!("unknown capability: {value}"),
    }
}

#[derive(Clone, Copy)]
struct TableRef<'a> {
    schema: &'a str,
    table: &'a str,
}

fn parse_table(table: &str) -> TableRef<'_> {
    if let Some((schema, tbl)) = table.split_once('.') {
        TableRef { schema, table: tbl }
    } else {
        TableRef {
            schema: "public",
            table,
        }
    }
}

async fn hook_add(args: HookAddArgs) -> Result<()> {
    let table = parse_table(&args.table);
    let secret = args.secret.unwrap_or_else(|| Uuid::new_v4().to_string());

    let pool = sqlx::PgPool::connect(&args.database_url)
        .await
        .with_context(|| "failed to connect to database")?;

    sqlx::query(
        "INSERT INTO flint.webhooks \
         (schema_name, table_name, events, target_url, secret, tier, target_type) \
         VALUES ($1, $2, $3, $4, $5, $6, 'url')",
    )
    .bind(table.schema)
    .bind(table.table)
    .bind(&args.events)
    .bind(&args.url)
    .bind(&secret)
    .bind(&args.tier)
    .execute(&pool)
    .await
    .with_context(|| "failed to insert webhook rule")?;

    let trigger_name = format!("flint_dispatch_{}_{}", table.schema, table.table);
    let sql = format!(
        "DROP TRIGGER IF EXISTS {trigger_name} ON {}.{}; \
         CREATE TRIGGER {trigger_name} \
         AFTER INSERT OR UPDATE OR DELETE ON {}.{} \
         FOR EACH ROW EXECUTE FUNCTION flint.dispatch_webhook();",
        table.schema, table.table, table.schema, table.table
    );
    sqlx::query(&sql)
        .execute(&pool)
        .await
        .with_context(|| "failed to create dispatch trigger")?;

    info!(
        schema = table.schema,
        table = table.table,
        url = %args.url,
        "added webhook"
    );
    println!("Webhook {}.{} -> {}", table.schema, table.table, args.url);
    Ok(())
}

async fn migrate(args: MigrateArgs) -> Result<()> {
    let pool = sqlx::PgPool::connect(&args.database_url)
        .await
        .with_context(|| "failed to connect to database")?;
    let source = args.source.clone();
    let migrator = sqlx::migrate::Migrator::new(source)
        .await
        .with_context(|| format!("failed to load migrations from {}", args.source.display()))?;
    migrator
        .run(&pool)
        .await
        .with_context(|| "migration failed")?;
    info!("migrations applied");
    println!("Migrations applied from {}", args.source.display());
    Ok(())
}

fn token_mint(args: TokenMintArgs) -> Result<()> {
    let secret = args
        .secret
        .with_context(|| "FLINT_JWT_SECRET or --secret required")?;
    let now = Utc::now();
    let claims = Claims {
        sub: args.subject,
        role: args.role,
        iat: now.timestamp(),
        exp: (now + Duration::seconds(args.expiry_seconds)).timestamp(),
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .with_context(|| "failed to encode JWT")?;
    println!("{token}");
    Ok(())
}

async fn run_in_container(cli: &Cli) -> Result<()> {
    let mut cmd = tokio::process::Command::new("docker");
    cmd.arg("run").arg("--rm").arg("-i");

    cmd.env(INSIDE_CONTAINER_ENV, "1");

    for key in [
        "DATABASE_URL",
        "KILN_ADMIN_URL",
        "FLINT_JWT_SECRET",
        "RUST_LOG",
        "RUST_BACKTRACE",
    ] {
        if let Ok(value) = std::env::var(key) {
            cmd.env(key, value);
        }
    }

    let cwd = std::env::current_dir().context("current directory not available")?;
    let cwd_str = cwd.to_string_lossy();
    cmd.arg("-v")
        .arg(format!("{cwd_str}:{cwd_str}"))
        .arg("-w")
        .arg(cwd_str.as_ref());

    if let Commands::Function {
        command: FunctionCommands::Register(args),
    } = &cli.command
    {
        if let Some(parent) = args.path.parent() {
            let parent_str = parent.to_string_lossy();
            cmd.arg("-v").arg(format!("{parent_str}:{parent_str}"));
        }
    }

    cmd.arg(CONTAINER_IMAGE);
    cmd.arg("forge");

    for token in std::env::args().skip(1) {
        if token == "--container" || token.starts_with("--container=") {
            continue;
        }
        cmd.arg(token);
    }

    let status = cmd
        .status()
        .await
        .with_context(|| "failed to run docker command")?;
    if !status.success() {
        bail!("container command failed with status {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
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
        let cli =
            Cli::try_parse_from(["forge", "hook", "add", "public.tasks", "https://example.com"])
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

    #[test]
    fn parse_table_with_schema() {
        let t = parse_table("myschema.mytable");
        assert_eq!(t.schema, "myschema");
        assert_eq!(t.table, "mytable");
    }

    #[test]
    fn parse_table_default_schema() {
        let t = parse_table("mytable");
        assert_eq!(t.schema, "public");
        assert_eq!(t.table, "mytable");
    }
}
