use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use chrono::{Duration, Utc};
use clap::{Args, Parser, Subcommand};
use fke_domain::{Capability, FunctionManifest};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
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
    /// Generate and rotate Flint project API keys.
    #[command(name = "keygen")]
    Keygen {
        #[command(subcommand)]
        command: KeygenCommands,
    },
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
    #[arg(
        short,
        long,
        value_delimiter = ',',
        default_value = "INSERT,UPDATE,DELETE"
    )]
    events: Vec<String>,
    /// Delivery tier: standard or durable.
    #[arg(short, long, default_value = "standard")]
    tier: String,
    /// Database URL.
    #[arg(
        long,
        env = "DATABASE_URL",
        default_value = "postgres://localhost/flint"
    )]
    database_url: String,
    /// Webhook HMAC secret. Generated if omitted.
    #[arg(long)]
    secret: Option<String>,
}

#[derive(Args)]
struct MigrateArgs {
    /// Database URL.
    #[arg(
        long,
        env = "DATABASE_URL",
        default_value = "postgres://localhost/flint"
    )]
    database_url: String,
    /// Migrations directory.
    #[arg(long, default_value = "migrations")]
    source: PathBuf,
}

#[derive(Subcommand)]
enum KeygenCommands {
    /// Initialize project anon/service-role keys.
    Init(KeygenInitArgs),
    /// Generate replacement signing material for rotation.
    Rotate(KeygenRotateArgs),
}

#[derive(Args)]
struct KeygenInitArgs {
    /// Output format: env, json, yaml, or shell.
    #[arg(short, long, default_value = "env")]
    format: String,
    /// JWT signing algorithm. HS256/HS384/HS512 are supported for local init.
    #[arg(short, long, default_value = "HS256")]
    algorithm: String,
    /// Flint project identifier.
    #[arg(short, long)]
    project: String,
    /// Deployment environment.
    #[arg(short, long, default_value = "development")]
    env: String,
    /// Issuer claim for generated JWTs.
    #[arg(long, default_value = "flint-forge")]
    issuer: String,
    /// Audience claim for generated JWTs.
    #[arg(long, default_value = "flint-forge")]
    audience: String,
    /// Optional output file.
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Overwrite output file if it exists.
    #[arg(long)]
    force: bool,
    /// Suppress security warnings.
    #[arg(short, long)]
    quiet: bool,
}

#[derive(Args)]
struct KeygenRotateArgs {
    /// Flint project identifier.
    #[arg(short, long)]
    project: String,
    /// Deployment environment.
    #[arg(short, long, default_value = "development")]
    env: String,
    /// JWT signing algorithm. HS256/HS384/HS512 are supported for local rotation.
    #[arg(short, long, default_value = "HS256")]
    algorithm: String,
    /// Grace-period marker for operator runbooks.
    #[arg(short, long, default_value = "168h")]
    grace_period: String,
    /// Output format: env, json, yaml, or shell.
    #[arg(short, long, default_value = "env")]
    format: String,
    /// Optional output file.
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Overwrite output file if it exists.
    #[arg(long)]
    force: bool,
    /// Suppress security warnings.
    #[arg(short, long)]
    quiet: bool,
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
    /// Principal type: User, Agent, or Service.
    #[arg(long, default_value = "User")]
    principal_type: String,
    /// JWT issuer.
    #[arg(long, default_value = "flint-forge")]
    issuer: String,
    /// JWT audience.
    #[arg(long, default_value = "flint-forge")]
    audience: String,
    /// Token expiry in seconds.
    #[arg(long, default_value_t = 3600)]
    expiry_seconds: i64,
    /// Optional tenant UUID claim.
    #[arg(long)]
    tenant_id: Option<String>,
    /// Optional session UUID claim.
    #[arg(long)]
    session_id: Option<String>,
    /// Space-delimited OAuth-style scope string.
    #[arg(long)]
    scope: Option<String>,
    /// Agent UUID claim for agent tokens.
    #[arg(long)]
    agent_id: Option<String>,
    /// Workflow UUID claim for agent tokens.
    #[arg(long)]
    workflow_id: Option<String>,
    /// Extra JSON object claims to merge into the token payload.
    #[arg(long, default_value = "{}")]
    claims: String,
}

#[derive(Serialize, Deserialize)]
struct RegisterBody {
    name: String,
    version: String,
    manifest: FunctionManifest,
    wasm_base64: String,
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
        Commands::Keygen {
            command: KeygenCommands::Init(args),
        } => keygen_init(args),
        Commands::Keygen {
            command: KeygenCommands::Rotate(args),
        } => keygen_rotate(args),
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
    let mut claims = parse_extra_claims(&args.claims)?;
    insert_claim(&mut claims, "sub", args.subject);
    insert_claim(&mut claims, "role", normalize_role(&args.role)?);
    insert_claim(
        &mut claims,
        "principal_type",
        normalize_principal_type(&args.principal_type)?,
    );
    insert_claim(&mut claims, "iss", args.issuer);
    insert_claim(&mut claims, "aud", args.audience);
    insert_claim(&mut claims, "iat", now.timestamp());
    insert_claim(
        &mut claims,
        "exp",
        (now + Duration::seconds(args.expiry_seconds)).timestamp(),
    );
    insert_claim(&mut claims, "jti", Uuid::new_v4().to_string());
    insert_optional_claim(&mut claims, "tenant_id", args.tenant_id);
    insert_optional_claim(&mut claims, "session_id", args.session_id);
    insert_optional_claim(&mut claims, "scope", args.scope);
    insert_optional_claim(&mut claims, "agent_id", args.agent_id);
    insert_optional_claim(&mut claims, "workflow_id", args.workflow_id);
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .with_context(|| "failed to encode JWT")?;
    println!("{token}");
    Ok(())
}

fn keygen_init(args: KeygenInitArgs) -> Result<()> {
    let algorithm = parse_hmac_algorithm(&args.algorithm)?;
    let secret = generate_signing_secret();
    let now = Utc::now();
    let expires = now + Duration::days(365 * 10);
    let anon_jti = format!("anon-key-{}-{}", args.project, args.env);
    let service_jti = format!("service-role-key-{}-{}", args.project, args.env);
    let anon_claims = fixed_project_claims(FixedProjectClaimInput {
        sub: "00000000-0000-0000-0000-000000000000",
        role: "anon",
        principal_type: "User",
        issuer: &args.issuer,
        audience: &args.audience,
        jti: &anon_jti,
        iat: now.timestamp(),
        exp: expires.timestamp(),
    });
    let service_claims = fixed_project_claims(FixedProjectClaimInput {
        sub: "00000000-0000-0000-0000-000000000001",
        role: "service_role",
        principal_type: "Service",
        issuer: &args.issuer,
        audience: &args.audience,
        jti: &service_jti,
        iat: now.timestamp(),
        exp: expires.timestamp(),
    });
    let anon_key = sign_claims(algorithm, &secret, &anon_claims)?;
    let service_role_key = sign_claims(algorithm, &secret, &service_claims)?;
    let material = ProjectKeys {
        project: args.project,
        env: args.env,
        algorithm: args.algorithm,
        jwt_secret: secret,
        anon_key,
        service_role_key,
        generated_at: now.to_rfc3339(),
    };
    write_key_output(&args.format, args.output.as_deref(), args.force, &material)?;
    if !args.quiet {
        print_key_warnings();
    }
    Ok(())
}

fn keygen_rotate(args: KeygenRotateArgs) -> Result<()> {
    parse_hmac_algorithm(&args.algorithm)?;
    let material = RotationKeys {
        project: args.project,
        env: args.env,
        algorithm: args.algorithm,
        jwt_secret: generate_signing_secret(),
        grace_period: args.grace_period,
        generated_at: Utc::now().to_rfc3339(),
    };
    write_rotation_output(&args.format, args.output.as_deref(), args.force, &material)?;
    if !args.quiet {
        eprintln!(
            "Rotation generated new signing material. Keep the old verifier active for {}.",
            material.grace_period
        );
    }
    Ok(())
}

#[derive(Serialize)]
struct ProjectKeys {
    project: String,
    env: String,
    algorithm: String,
    jwt_secret: String,
    anon_key: String,
    service_role_key: String,
    generated_at: String,
}

#[derive(Serialize)]
struct RotationKeys {
    project: String,
    env: String,
    algorithm: String,
    jwt_secret: String,
    grace_period: String,
    generated_at: String,
}

#[derive(Clone, Copy)]
struct FixedProjectClaimInput<'a> {
    sub: &'a str,
    role: &'a str,
    principal_type: &'a str,
    issuer: &'a str,
    audience: &'a str,
    jti: &'a str,
    iat: i64,
    exp: i64,
}

fn fixed_project_claims(input: FixedProjectClaimInput<'_>) -> Map<String, Value> {
    let mut claims = Map::new();
    insert_claim(&mut claims, "sub", input.sub.to_owned());
    insert_claim(&mut claims, "role", input.role.to_owned());
    insert_claim(
        &mut claims,
        "principal_type",
        input.principal_type.to_owned(),
    );
    insert_claim(&mut claims, "iss", input.issuer.to_owned());
    insert_claim(&mut claims, "aud", input.audience.to_owned());
    insert_claim(&mut claims, "iat", input.iat);
    insert_claim(&mut claims, "exp", input.exp);
    insert_claim(&mut claims, "jti", input.jti.to_owned());
    claims
}

fn sign_claims(algorithm: Algorithm, secret: &str, claims: &Map<String, Value>) -> Result<String> {
    let mut header = Header::new(algorithm);
    header.typ = Some("JWT".to_owned());
    encode(&header, claims, &EncodingKey::from_secret(secret.as_bytes()))
        .with_context(|| "failed to encode JWT")
}

fn parse_hmac_algorithm(value: &str) -> Result<Algorithm> {
    match value.to_ascii_uppercase().as_str() {
        "HS256" => Ok(Algorithm::HS256),
        "HS384" => Ok(Algorithm::HS384),
        "HS512" => Ok(Algorithm::HS512),
        other => bail!("{other} is not supported for local keygen; use HS256, HS384, or HS512"),
    }
}

fn generate_signing_secret() -> String {
    let mut bytes = [0_u8; 32];
    bytes[..16].copy_from_slice(Uuid::new_v4().as_bytes());
    bytes[16..].copy_from_slice(Uuid::new_v4().as_bytes());
    general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn parse_extra_claims(raw: &str) -> Result<Map<String, Value>> {
    let value: Value = serde_json::from_str(raw).with_context(|| "--claims must be valid JSON")?;
    match value {
        Value::Object(map) => Ok(map),
        _ => bail!("--claims must be a JSON object"),
    }
}

fn normalize_role(role: &str) -> Result<String> {
    match role {
        "anon" | "authenticated" | "agent" | "service_role" => Ok(role.to_owned()),
        other => bail!("unsupported role {other}; expected anon, authenticated, agent, service_role"),
    }
}

fn normalize_principal_type(value: &str) -> Result<String> {
    match value {
        "User" | "Agent" | "Service" => Ok(value.to_owned()),
        other => bail!("unsupported principal type {other}; expected User, Agent, or Service"),
    }
}

fn insert_claim<T: Serialize>(claims: &mut Map<String, Value>, key: &str, value: T) {
    claims.insert(key.to_owned(), json!(value));
}

fn insert_optional_claim(claims: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if let Some(value) = value {
        insert_claim(claims, key, value);
    }
}

fn write_key_output(
    format: &str,
    output: Option<&Path>,
    force: bool,
    material: &ProjectKeys,
) -> Result<()> {
    let rendered = render_project_keys(format, material)?;
    write_or_print(output, force, &rendered)
}

fn write_rotation_output(
    format: &str,
    output: Option<&Path>,
    force: bool,
    material: &RotationKeys,
) -> Result<()> {
    let rendered = render_rotation_keys(format, material)?;
    write_or_print(output, force, &rendered)
}

fn write_or_print(output: Option<&Path>, force: bool, rendered: &str) -> Result<()> {
    if let Some(path) = output {
        if path.exists() && !force {
            bail!("output file {} exists; pass --force to overwrite", path.display());
        }
        std::fs::write(path, rendered)
            .with_context(|| format!("failed to write {}", path.display()))?;
    } else {
        println!("{rendered}");
    }
    Ok(())
}

fn render_project_keys(format: &str, material: &ProjectKeys) -> Result<String> {
    match format {
        "env" | "shell" => Ok(format!(
            "# Prometheus Flint Project Keys\n\
             # Project: {} | Environment: {}\n\
             # Generated: {}\n\
             FLINT_JWT_SECRET=\"{}\"\n\
             FLINT_JWT_ALGORITHM=\"{}\"\n\
             FLINT_ANON_KEY=\"{}\"\n\
             FLINT_SERVICE_ROLE_KEY=\"{}\"\n\
             FLINT_PROJECT_ID=\"{}\"\n\
             FLINT_ENV=\"{}\"\n",
            material.project,
            material.env,
            material.generated_at,
            material.jwt_secret,
            material.algorithm,
            material.anon_key,
            material.service_role_key,
            material.project,
            material.env
        )),
        "json" => serde_json::to_string_pretty(material).with_context(|| "failed to render JSON"),
        "yaml" => Ok(format!(
            "project: \"{}\"\n\
             env: \"{}\"\n\
             algorithm: \"{}\"\n\
             jwt_secret: \"{}\"\n\
             anon_key: \"{}\"\n\
             service_role_key: \"{}\"\n\
             generated_at: \"{}\"\n",
            material.project,
            material.env,
            material.algorithm,
            material.jwt_secret,
            material.anon_key,
            material.service_role_key,
            material.generated_at
        )),
        other => bail!("unsupported output format {other}; expected env, json, yaml, or shell"),
    }
}

fn render_rotation_keys(format: &str, material: &RotationKeys) -> Result<String> {
    match format {
        "env" | "shell" => Ok(format!(
            "# Prometheus Flint JWT Signing Key Rotation\n\
             # Project: {} | Environment: {}\n\
             # Generated: {}\n\
             FLINT_NEXT_JWT_SECRET=\"{}\"\n\
             FLINT_JWT_ALGORITHM=\"{}\"\n\
             FLINT_KEY_GRACE_PERIOD=\"{}\"\n",
            material.project,
            material.env,
            material.generated_at,
            material.jwt_secret,
            material.algorithm,
            material.grace_period
        )),
        "json" => serde_json::to_string_pretty(material).with_context(|| "failed to render JSON"),
        "yaml" => Ok(format!(
            "project: \"{}\"\n\
             env: \"{}\"\n\
             algorithm: \"{}\"\n\
             next_jwt_secret: \"{}\"\n\
             grace_period: \"{}\"\n\
             generated_at: \"{}\"\n",
            material.project,
            material.env,
            material.algorithm,
            material.jwt_secret,
            material.grace_period,
            material.generated_at
        )),
        other => bail!("unsupported output format {other}; expected env, json, yaml, or shell"),
    }
}

fn print_key_warnings() {
    eprintln!(
        "\nSECURITY WARNING: FLINT_SERVICE_ROLE_KEY bypasses Row-Level Security. \
         Never expose it to clients, browsers, or public repositories."
    );
    eprintln!(
        "FLINT_ANON_KEY is publishable only when RLS policies are correctly configured.\n"
    );
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
    fn parse_keygen_init_defaults() {
        let cli = Cli::try_parse_from(["forge", "keygen", "init", "--project", "acme"]).unwrap();
        match cli.command {
            Commands::Keygen {
                command: KeygenCommands::Init(args),
            } => {
                assert_eq!(args.project, "acme");
                assert_eq!(args.env, "development");
                assert_eq!(args.algorithm, "HS256");
                assert_eq!(args.format, "env");
            }
            _ => panic!("expected keygen init"),
        }
    }

    #[test]
    fn parse_keygen_rotate_defaults() {
        let cli = Cli::try_parse_from(["forge", "keygen", "rotate", "--project", "acme"]).unwrap();
        match cli.command {
            Commands::Keygen {
                command: KeygenCommands::Rotate(args),
            } => {
                assert_eq!(args.project, "acme");
                assert_eq!(args.grace_period, "168h");
            }
            _ => panic!("expected keygen rotate"),
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
                assert_eq!(args.role, "authenticated");
                assert_eq!(args.principal_type, "User");
                assert_eq!(args.expiry_seconds, 3600);
            }
            _ => panic!("expected token mint"),
        }
    }

    #[test]
    fn parse_token_mint_agent_claims() {
        let cli = Cli::try_parse_from([
            "forge",
            "token",
            "mint",
            "--secret",
            "x",
            "--role",
            "agent",
            "--principal-type",
            "Agent",
            "--agent-id",
            "agent-1",
            "--workflow-id",
            "wf-1",
            "--scope",
            "read write",
        ])
        .unwrap();
        match cli.command {
            Commands::Token {
                command: TokenCommands::Mint(args),
            } => {
                assert_eq!(args.role, "agent");
                assert_eq!(args.principal_type, "Agent");
                assert_eq!(args.agent_id.as_deref(), Some("agent-1"));
                assert_eq!(args.workflow_id.as_deref(), Some("wf-1"));
                assert_eq!(args.scope.as_deref(), Some("read write"));
            }
            _ => panic!("expected token mint"),
        }
    }

    #[test]
    fn render_project_keys_env_contains_expected_names() {
        let material = ProjectKeys {
            project: "acme".to_owned(),
            env: "production".to_owned(),
            algorithm: "HS256".to_owned(),
            jwt_secret: "secret".to_owned(),
            anon_key: "anon.jwt".to_owned(),
            service_role_key: "service.jwt".to_owned(),
            generated_at: "2026-07-08T00:00:00Z".to_owned(),
        };
        let rendered = render_project_keys("env", &material).unwrap();
        assert!(rendered.contains("FLINT_ANON_KEY=\"anon.jwt\""));
        assert!(rendered.contains("FLINT_SERVICE_ROLE_KEY=\"service.jwt\""));
    }

    #[test]
    fn fixed_project_claims_sets_role_and_principal_type() {
        let claims = fixed_project_claims(FixedProjectClaimInput {
            sub: "sub",
            role: "service_role",
            principal_type: "Service",
            issuer: "issuer",
            audience: "aud",
            jti: "jti",
            iat: 1,
            exp: 2,
        });
        assert_eq!(claims["role"], "service_role");
        assert_eq!(claims["principal_type"], "Service");
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
