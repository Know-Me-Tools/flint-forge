//! `forge hook add` — add a webhook dispatch rule for a table.

use anyhow::{Context, Result};
use clap::Args;
use tracing::info;
use uuid::Uuid;

#[derive(Args)]
pub struct HookAddArgs {
    /// Target table as [schema.]table.
    pub table: String,
    /// Webhook URL.
    pub url: String,
    /// Events to subscribe to (comma-separated).
    #[arg(
        short,
        long,
        value_delimiter = ',',
        default_value = "INSERT,UPDATE,DELETE"
    )]
    pub events: Vec<String>,
    /// Delivery tier: standard or durable.
    #[arg(short, long, default_value = "standard")]
    pub tier: String,
    /// Database URL.
    #[arg(
        long,
        env = "DATABASE_URL",
        default_value = "postgres://localhost/flint"
    )]
    pub database_url: String,
    /// Webhook HMAC secret. Generated if omitted.
    #[arg(long)]
    pub secret: Option<String>,
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

pub async fn hook_add(args: HookAddArgs) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
