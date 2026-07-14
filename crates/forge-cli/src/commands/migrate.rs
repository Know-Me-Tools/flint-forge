//! `forge migrate` — apply SQL migrations.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use tracing::info;

#[derive(Args)]
pub struct MigrateArgs {
    /// Database URL.
    #[arg(
        long,
        env = "DATABASE_URL",
        default_value = "postgres://localhost/flint"
    )]
    pub database_url: String,
    /// Migrations directory.
    #[arg(long, default_value = "migrations")]
    pub source: PathBuf,
}

pub async fn migrate(args: MigrateArgs) -> Result<()> {
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
