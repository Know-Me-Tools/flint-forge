//! Live-Postgres integration test: REST filters, PATCH SET, and embed joins
//! against NON-text columns (`int4`/`int8`/`bool`/`uuid`), not just `text`.
//!
//! DATABASE_URL-gated: runs when `DATABASE_URL` is set, skips otherwise, so the
//! default `cargo test` / CI unit stage never require a database.
//!
//! # Why this test exists
//! Every pre-existing unit test builds columns via a `col()` helper that always
//! sets `pg_type: "text"` (see `compilers::rest::mod::tests::col` and
//! `compilers::embed_schema::tests::col`), so the WHERE/SET/embed-filter
//! rendering path was never unit-tested against the common case: typed
//! (int/bigint/bool/uuid) primary/foreign keys and scalar columns. Binding a
//! value as `sqlx`'s default `String` (`text`) against those columns fails
//! server-side (`operator does not exist: integer = text`) unless the bound
//! placeholder carries an explicit `::pg_type` cast — this test exercises the
//! exact SQL/bind mechanics `handle_list`/`handle_update`/`handle_delete` use
//! (via `compilers::filters`) against a real ephemeral schema with those types.
//!
//! (The REST HTTP router itself cannot be driven end-to-end today — a separate,
//! pre-existing bug in `endpoint_generation`'s route registration vs. the
//! handlers' `Path<(String, String)>` extractor arity makes every real HTTP
//! request 500 regardless of this fix. This test exercises the same
//! query-builder + real-`sqlx`-execution layer `pgrest_live_pg.rs` and
//! `embedding_live_pg.rs` already use for that reason.)
#![allow(clippy::expect_used)]

use std::collections::HashMap;

use fdb_query::embed::{
    parse_embed_select, render_projection, resolve_embeds, route_embedded_param,
};
use fdb_query::Select;
use fdb_reflection::compilers::embed_schema::embed_schema_from_model;
use fdb_reflection::compilers::filters::{
    bind_mutation_value, bind_param, cast_hints_for, mutation_placeholder, mutation_value_to_bind,
    parse_filter_tree, render_where_with_hints,
};
use fdb_reflection::model::{Column, DatabaseModel, ForeignKey, Table};
use fdb_reflection::passes::normalization;
use sqlx::Row;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty())
}

fn col(name: &str, pg_type: &str) -> Column {
    Column {
        name: name.into(),
        pg_type: pg_type.into(),
        nullable: true,
        default: None,
    }
}

/// customers(id int4, name text) 1—* orders(id int4, customer_id int4, total
/// int8, active bool, ext_id uuid). Built with RAW introspected type names and
/// run through the real normalization pass, so `pg_type` matches exactly what
/// production reflection would produce (`int4`→`integer`, `int8`→`bigint`,
/// `bool`→`boolean`).
///
/// `schema` is unique per test — `#[tokio::test]` functions in this file run
/// concurrently against the SAME database, so each needs its own ephemeral
/// schema to avoid a `CREATE SCHEMA` race.
fn typed_model(schema: &str) -> DatabaseModel {
    let mut model = DatabaseModel {
        tables: vec![
            Table {
                schema: schema.into(),
                name: "customers".into(),
                columns: vec![col("id", "int4"), col("name", "text")],
                pk: vec!["id".into()],
                fk: vec![],
                rls_enabled: true,
                vault_key: None,
            },
            Table {
                schema: schema.into(),
                name: "orders".into(),
                columns: vec![
                    col("id", "int4"),
                    col("customer_id", "int4"),
                    col("total", "int8"),
                    col("active", "bool"),
                    col("ext_id", "uuid"),
                ],
                pk: vec!["id".into()],
                fk: vec![ForeignKey {
                    from_col: "customer_id".into(),
                    to_schema: schema.into(),
                    to_table: "customers".into(),
                    to_col: "id".into(),
                }],
                rls_enabled: true,
                vault_key: None,
            },
        ],
        functions: vec![],
        views: vec![],
        version: 1,
    };
    normalization::run(&mut model);
    model
}

const EXT_ID: &str = "11111111-1111-1111-1111-111111111111";

async fn setup(pool: &sqlx::PgPool, schema: &str) {
    // SAFETY: `schema` is always a hardcoded literal passed by each test
    // function below (e.g. "typed_it_where"), never external input.
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "DROP SCHEMA IF EXISTS {schema} CASCADE; \
         CREATE SCHEMA {schema}; \
         CREATE TABLE {schema}.customers (id int4 PRIMARY KEY, name text); \
         CREATE TABLE {schema}.orders ( \
             id int4 PRIMARY KEY, \
             customer_id int4 REFERENCES {schema}.customers(id), \
             total int8, active bool, ext_id uuid); \
         INSERT INTO {schema}.customers VALUES (1, 'acme'); \
         INSERT INTO {schema}.orders VALUES \
             (10, 1, 50, false, '{EXT_ID}'), \
             (11, 1, 150, true, '{EXT_ID}');"
    )))
    .execute(pool)
    .await
    .expect("ephemeral schema");
}

async fn teardown(pool: &sqlx::PgPool, schema: &str) {
    // SAFETY: `schema` is always a hardcoded literal — see `setup` above.
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "DROP SCHEMA {schema} CASCADE;"
    )))
    .execute(pool)
    .await
    .expect("cleanup");
}

fn params(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

/// `?total=gt.100&active=eq.true` against int8/bool columns — the exact SQL
/// shape `handle_list` renders for a WHERE clause, executed for real.
#[tokio::test]
async fn where_filter_against_int8_and_bool_columns() {
    let Some(url) = database_url() else {
        eprintln!("[rest_typed_columns_live_pg] DATABASE_URL unset — skipping");
        return;
    };
    let pool = sqlx::PgPool::connect(&url).await.expect("connect");
    let schema = "typed_it_where";
    setup(&pool, schema).await;

    let model = typed_model(schema);
    let hints = cast_hints_for(&model, schema, "orders");
    let tree =
        parse_filter_tree(&params(&[("total", "gt.100"), ("active", "eq.true")])).expect("parse");
    let wc = render_where_with_hints(&tree, 1, &hints).expect("render");
    assert!(
        wc.sql.contains("::bigint") || wc.sql.contains("::boolean"),
        "expected a cast on a non-text column, got: {}",
        wc.sql
    );

    let sql = format!("SELECT id FROM {schema}.orders {}", wc.sql);
    // SAFETY: `schema` is a hardcoded literal; `wc.sql` is engine-rendered
    // WHERE SQL with all values bound as `$n` (asserted above via casts, never
    // interpolated).
    let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
    for b in &wc.binds {
        q = bind_param(q, b);
    }
    let rows = q.fetch_all(&pool).await.expect("typed filter query");
    let ids: Vec<i32> = rows.iter().map(|r| r.get("id")).collect();
    assert_eq!(ids, vec![11], "only order 11 has total>100 AND active=true");

    teardown(&pool, schema).await;
}

/// `?ext_id=eq.<uuid>` against a `uuid` column.
#[tokio::test]
async fn where_filter_against_uuid_column() {
    let Some(url) = database_url() else {
        eprintln!("[rest_typed_columns_live_pg] DATABASE_URL unset — skipping");
        return;
    };
    let pool = sqlx::PgPool::connect(&url).await.expect("connect");
    let schema = "typed_it_uuid";
    setup(&pool, schema).await;

    let model = typed_model(schema);
    let hints = cast_hints_for(&model, schema, "orders");
    let tree = parse_filter_tree(&params(&[("ext_id", &format!("eq.{EXT_ID}"))])).expect("parse");
    let wc = render_where_with_hints(&tree, 1, &hints).expect("render");
    assert!(
        wc.sql.contains("::uuid"),
        "expected a uuid cast, got: {}",
        wc.sql
    );

    let sql = format!("SELECT count(*) AS n FROM {schema}.orders {}", wc.sql);
    // SAFETY: `schema` is a hardcoded literal; `wc.sql` is engine-rendered
    // WHERE SQL with all values bound as `$n`, never interpolated.
    let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
    for b in &wc.binds {
        q = bind_param(q, b);
    }
    let row = q.fetch_one(&pool).await.expect("uuid filter query");
    let n: i64 = row.get("n");
    assert_eq!(n, 2, "both orders share the same ext_id");

    teardown(&pool, schema).await;
}

/// PATCH-shaped `SET active = $1 WHERE id = $2` against bool/int4 columns —
/// the exact bind mechanics `handle_update` uses for the JSON body + filter.
#[tokio::test]
async fn patch_set_against_bool_and_int4_columns() {
    let Some(url) = database_url() else {
        eprintln!("[rest_typed_columns_live_pg] DATABASE_URL unset — skipping");
        return;
    };
    let pool = sqlx::PgPool::connect(&url).await.expect("connect");
    let schema = "typed_it_patch";
    setup(&pool, schema).await;

    let model = typed_model(schema);
    let hints = cast_hints_for(&model, schema, "orders");

    let bind = mutation_value_to_bind(&serde_json::json!(true));
    let set_sql = format!(
        "active = {}",
        mutation_placeholder(1, &bind, hints.get("active"))
    );
    assert_eq!(set_sql, "active = $1::boolean");

    let tree = parse_filter_tree(&params(&[("id", "eq.10")])).expect("parse");
    let wc = render_where_with_hints(&tree, 2, &hints).expect("render");
    assert_eq!(wc.sql, "WHERE id = $2::integer");

    let sql = format!("UPDATE {schema}.orders SET {set_sql} {}", wc.sql);
    // SAFETY: `schema` is a hardcoded literal; `set_sql`/`wc.sql` are
    // engine-rendered with all values bound as `$n`, never interpolated.
    let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
    q = bind_mutation_value(q, &bind);
    for b in &wc.binds {
        q = bind_param(q, b);
    }
    let result = q.execute(&pool).await.expect("typed PATCH SET");
    assert_eq!(result.rows_affected(), 1);

    // SAFETY: `schema` is a hardcoded literal; the rest of the query is a
    // fixed string literal with no interpolated values.
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT active FROM {schema}.orders WHERE id = 10"
    )))
    .fetch_one(&pool)
    .await
    .expect("verify");
    let active: bool = row.get("active");
    assert!(active, "order 10's active flag was flipped to true");

    teardown(&pool, schema).await;
}

/// `?select=*,orders(*)&orders.total=gt.100` — an embed-scoped filter against
/// the CHILD table's `int8` column, qualified with its correlation alias.
#[tokio::test]
async fn embed_filter_against_typed_child_column() {
    let Some(url) = database_url() else {
        eprintln!("[rest_typed_columns_live_pg] DATABASE_URL unset — skipping");
        return;
    };
    let pool = sqlx::PgPool::connect(&url).await.expect("connect");
    let schema = "typed_it_embed";
    setup(&pool, schema).await;

    let model = typed_model(schema);
    let embed_schema = embed_schema_from_model(&model);

    let mut sel = parse_embed_select("*,orders(*)").expect("parse select");
    route_embedded_param(&mut sel, "orders.total", "gt.100").expect("route");
    let resolved = resolve_embeds(&sel, "customers", "customers", &embed_schema).expect("resolve");
    let (projection, params_out, _) =
        render_projection(&Select::default(), &resolved, 1).expect("render");
    assert!(
        projection.contains("::bigint"),
        "embed-scoped filter on orders.total (int8/bigint) must be cast: {projection}"
    );

    let mut conn = pool.acquire().await.expect("acquire");
    // SAFETY: `schema` is a hardcoded literal — see `setup` above.
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}")))
        .execute(&mut *conn)
        .await
        .expect("set search_path");

    let projection = projection.replacen('*', "customers.*", 1);
    let sql = format!("SELECT {projection} FROM customers customers");
    // SAFETY: `projection` is the engine's own rendered projection over a
    // fixed test schema; all filter values are bound as `$n` via `params_out`.
    let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
    for p in &params_out {
        q = bind_param(q, p);
    }
    let rows = q.fetch_all(&mut *conn).await.expect("run embed sql");
    assert_eq!(rows.len(), 1, "one customer");
    let orders_json: serde_json::Value = rows[0].try_get("orders").expect("orders col");
    let arr = orders_json.as_array().expect("orders is a JSON array");
    assert_eq!(arr.len(), 1, "only order 11 (total=150) matches total>100");

    drop(conn);
    teardown(&pool, schema).await;
}
