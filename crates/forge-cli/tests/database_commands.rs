//! Verify that operator migrations and hook registration work together on PostgreSQL.
use anyhow::{Context, Result};
use std::process::Command;
use tempfile::tempdir;

#[tokio::test]
async fn applies_migrations_and_registers_a_hook_in_a_fresh_database() -> Result<()> {
    let Ok(admin_url) = std::env::var("DATABASE_URL") else {
        return Ok(());
    };
    let admin = sqlx::PgPool::connect(&admin_url).await?;
    let name = format!("forge_cli_{}", uuid::Uuid::new_v4().simple());
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "CREATE DATABASE {name} TEMPLATE template0"
    )))
    .execute(&admin)
    .await?;
    let mut url = reqwest::Url::parse(&admin_url)?;
    url.set_path(&format!("/{name}"));
    let result = exercise(url.as_str()).await;
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "DROP DATABASE {name} WITH (FORCE)"
    )))
    .execute(&admin)
    .await?;
    result
}

async fn exercise(url: &str) -> Result<()> {
    let dir = tempdir()?;
    std::fs::write(
        dir.path().join("0001_fixture.sql"),
        "CREATE EXTENSION \"ext-flint-hooks\" CASCADE;
         CREATE TABLE public.widgets (id bigint PRIMARY KEY);",
    )?;
    let cli = || {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_forge"));
        cmd.env("DATABASE_URL", url)
            .env_remove("FORGE_CONTAINER")
            .env("RUST_LOG", "off");
        cmd
    };
    for _ in 0..2 {
        let output = cli()
            .args(["migrate", "--source"])
            .arg(dir.path())
            .output()?;
        anyhow::ensure!(
            output.status.success(),
            "migration failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let output = cli()
        .args([
            "hook",
            "add",
            "public.widgets",
            "https://fixture.invalid/callback",
            "--events",
            "INSERT",
            "--tier",
            "durable",
        ])
        .output()?;
    anyhow::ensure!(
        output.status.success(),
        "hook registration failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let pool = sqlx::PgPool::connect(url).await?;
    let (events, tier, bound): (Vec<String>, String, bool) = sqlx::query_as(
        "SELECT events, tier, EXISTS (
            SELECT 1 FROM pg_trigger WHERE tgrelid = 'public.widgets'::regclass
            AND tgname = 'flint_dispatch_public_widgets'
         ) FROM flint.webhooks WHERE schema_name = 'public' AND table_name = 'widgets'",
    )
    .fetch_one(&pool)
    .await
    .context("registered hook")?;
    anyhow::ensure!(
        bound && events == ["INSERT"] && tier == "durable",
        "database does not contain the registered hook"
    );
    let missing_table = cli()
        .args([
            "hook",
            "add",
            "public.absent",
            "https://fixture.invalid/callback",
        ])
        .output()?;
    anyhow::ensure!(
        !missing_table.status.success(),
        "registered a hook on an absent table"
    );
    let orphan: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM flint.webhooks WHERE table_name = 'absent')",
    )
    .fetch_one(&pool)
    .await?;
    anyhow::ensure!(!orphan, "failed registration persisted a webhook rule");
    pool.close().await;
    Ok(())
}
