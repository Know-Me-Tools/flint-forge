//! Live-Postgres integration test for resource-embedding SQL generation (p35-c004).
//!
//! DATABASE_URL-gated: runs when `DATABASE_URL` is set, skips otherwise, so the
//! default `cargo test` / CI unit stage never require a database.
//!
//! Proves that the SQL the embedding engine generates is VALID Postgres and yields
//! the expected nested JSON. It exercises the public `fdb_query::embed` API
//! (`parse_embed_select` → `resolve_embeds` → `render_projection`) — the same
//! rendering the REST list handler uses — then runs the generated projection
//! against a real parent/child FK schema and asserts the embedded child array.
#![allow(clippy::expect_used)]

use fdb_query::embed::{
    parse_embed_select, render_projection, resolve_embeds, Cardinality, EmbedSchema, FkEdge,
    TableDesc,
};
use fdb_query::Select;
use sqlx::Row;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty())
}

/// customers 1—* orders (orders.customer_id -> customers.id).
fn schema() -> EmbedSchema {
    // Mirrors embed_schema_from_model's to-many edge: the referenced table
    // (customers) carries an edge describing orders.customer_id -> customers.id.
    // pick_edge matches on (from_table==target && to_table==parent).
    let customers = TableDesc::new()
        .with_column("id")
        .with_column("name")
        .with_fk(FkEdge {
            fk_name: "orders_customer_id_fkey".into(),
            from_table: "orders".into(),
            from_col: "customer_id".into(),
            to_table: "customers".into(),
            to_col: "id".into(),
            cardinality: Cardinality::ToMany,
        });
    let orders = TableDesc::new()
        .with_column("id")
        .with_column("customer_id")
        .with_column("total");
    EmbedSchema::new()
        .with_table("customers", customers)
        .with_table("orders", orders)
}

#[tokio::test]
async fn embedding_projection_sql_yields_nested_json() {
    let Some(url) = database_url() else {
        eprintln!("[embedding_live_pg] DATABASE_URL unset — skipping");
        return;
    };
    let pool = sqlx::PgPool::connect(&url).await.expect("connect");

    // raw_sql runs the multi-statement setup via the simple-query protocol
    // (`sqlx::query` prepares a statement, which rejects multiple commands).
    sqlx::raw_sql(
        "DROP SCHEMA IF EXISTS embed_it CASCADE; \
         CREATE SCHEMA embed_it; \
         CREATE TABLE embed_it.customers (id int PRIMARY KEY, name text); \
         CREATE TABLE embed_it.orders (id int PRIMARY KEY, customer_id int REFERENCES embed_it.customers(id), total int); \
         INSERT INTO embed_it.customers VALUES (1,'acme'); \
         INSERT INTO embed_it.orders VALUES (10,1,100),(11,1,250);",
    )
    .execute(&pool)
    .await
    .expect("ephemeral schema");

    // Generate the embedding projection via the public engine API: customers embedding orders.
    let sel = parse_embed_select("*,orders(*)").expect("parse select");
    let resolved = resolve_embeds(&sel, "customers", "customers", &schema()).expect("resolve");
    let (projection, params, _) =
        render_projection(&Select::default(), &resolved, 1).expect("render");
    assert!(params.is_empty(), "no filters → no binds in this case");

    // The embed subselects emit unqualified table names (`FROM orders`), relying on
    // the connection's search_path for schema resolution (as the reflection handler's
    // per-request context does). Acquire ONE connection and set search_path on it, then
    // run the query on the SAME connection (a pool would hand out a different conn).
    let mut conn = pool.acquire().await.expect("acquire");
    sqlx::raw_sql("SET search_path TO embed_it")
        .execute(&mut *conn)
        .await
        .expect("set search_path");

    // Qualify the base `*` to the parent alias (as the REST handler does) and run it.
    let projection = projection.replacen('*', "customers.*", 1);
    let sql = format!("SELECT {projection} FROM customers customers");

    // SAFETY: `sql` is built from the engine's own rendered projection over a
    // fixed test schema — no external input reaches this string.
    let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
        .fetch_all(&mut *conn)
        .await
        .expect("run embed sql");
    assert_eq!(rows.len(), 1, "one customer");

    // The embedded `orders` column is a JSON array of the two orders.
    let orders_json: serde_json::Value = rows[0].try_get("orders").expect("orders col");
    let arr = orders_json.as_array().expect("orders is a JSON array");
    assert_eq!(arr.len(), 2, "customer 1 has two embedded orders");
    let totals: Vec<i64> = arr
        .iter()
        .filter_map(|o| o.get("total").and_then(serde_json::Value::as_i64))
        .collect();
    assert!(totals.contains(&100) && totals.contains(&250));

    sqlx::query("DROP SCHEMA embed_it CASCADE;")
        .execute(&pool)
        .await
        .expect("cleanup");
}
