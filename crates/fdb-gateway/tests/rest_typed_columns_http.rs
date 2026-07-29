//! p17-c005 gate test: typed (non-`text`) columns over the **HTTP handler
//! path**, for all four REST verbs.
//!
//! `DATABASE_URL`-gated; also requires the `authenticated` role (provisioned by
//! the `ext-flint-auth` pgrx extension in the CI Postgres image). Skips cleanly
//! if either is absent, so `cargo test --workspace` never requires a database.
//!
//! # Why this test exists
//!
//! `fdb-reflection`'s `rest_typed_columns_live_pg.rs` already proves the
//! *query-builder* layer casts correctly (via `CastHints`,
//! `render_where_with_hints`, and `mutation_placeholder`). What nothing covered
//! is whether the REST **handlers** actually call any of it — and they did not.
//! Every list, update, and delete handler invoked the UNCAST `render_where`, so
//! a filter on any non-`text` column failed at the driver's
//! parameter-type-inference step. The insert and update handlers separately
//! hand-rolled a weaker `placeholder_for` that disagreed with `json_bind` about
//! the bind channel for JSON numbers and booleans.
//!
//! Two existing tests each sidestepped the seam rather than covering it.
//! `rest_rls_isolation.rs` uses `text` for every column (see its own comment at
//! the `seed_schema` helper). `rest_typed_columns_live_pg.rs` drops to the
//! query-builder layer, citing a route-registration bug that p16-c001 has since
//! fixed. This test closes the gap by driving the real compiled `RestCompiler`
//! router end-to-end — the same code path production traffic takes.
//!
//! Every assertion below fails against the pre-p17 handlers.

#![allow(clippy::expect_used)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use deadpool_postgres::{Config, Runtime};
use fdb_ports::SqlExecutor;
use fdb_postgres::PgRest;
use fdb_reflection::{
    compilers::rest::RestCompiler,
    model::{Column, DatabaseModel, Table},
    passes::normalization,
};
use forge_identity::RlsContext;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio_postgres::NoTls;
use tower::ServiceExt;

const SCHEMA: &str = "p17c005_http";

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty())
}

fn rls() -> RlsContext {
    RlsContext {
        role: "authenticated".into(),
        claims_json: r#"{"sub":"p17c005-user"}"#.into(),
        raw_bearer: "test-token".into(),
        keto_subject: "p17c005-user".into(),
        vault_key_id: None,
    }
}

fn col(name: &str, pg_type: &str) -> Column {
    Column {
        name: name.into(),
        pg_type: pg_type.into(),
        nullable: true,
        default: None,
    }
}

/// One row type per previously-unwritable Postgres type, plus `text`/`jsonb` as
/// must-not-regress controls.
///
/// Built from RAW introspected type names and run through the real
/// normalization pass, so `pg_type` matches exactly what production reflection
/// produces (`int4`→`integer`, `varchar`→`character varying`, …). That matters:
/// the cast string emitted downstream is the *canonicalized* name, and a
/// canonical form that is not a valid cast target would only fail here.
fn model() -> DatabaseModel {
    let mut model = DatabaseModel {
        tables: vec![Table {
            schema: SCHEMA.into(),
            name: "widgets".into(),
            columns: vec![
                col("id", "uuid"),
                col("qty", "int4"),
                col("big", "int8"),
                col("price", "numeric"),
                col("active", "bool"),
                col("received_on", "date"),
                col("seen_at", "timestamptz"),
                col("ratio", "float8"),
                col("label", "varchar"),
                col("note", "text"),
                col("meta", "jsonb"),
            ],
            pk: vec!["id".into()],
            fk: vec![],
            rls_enabled: true,
            vault_key: None,
        }],
        functions: vec![],
        views: vec![],
        version: 1,
    };
    normalization::run(&mut model);
    model
}

const ROW_A: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const ROW_B: &str = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";

async fn send(
    router: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    let body_bytes = body.map_or_else(String::new, |v| v.to_string());
    let mut req = builder.body(Body::from(body_bytes)).expect("build request");
    req.extensions_mut().insert(rls());

    let resp = router.clone().oneshot(req).await.expect("request");
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("read body");
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, json)
}

/// Returns `false` (skip) when `authenticated` doesn't exist — `ext-flint-auth`
/// is not installed in this Postgres.
async fn role_authenticated_present(setup: &deadpool_postgres::Object) -> bool {
    setup
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'authenticated')",
            &[],
        )
        .await
        .expect("role check")
        .get(0)
}

async fn seed_schema(setup: &deadpool_postgres::Object) {
    setup
        .batch_execute(&format!(
            "DROP SCHEMA IF EXISTS {SCHEMA} CASCADE; \
             CREATE SCHEMA {SCHEMA}; \
             CREATE TABLE {SCHEMA}.widgets ( \
                 id uuid PRIMARY KEY, \
                 qty int4, \
                 big int8, \
                 price numeric, \
                 active bool, \
                 received_on date, \
                 seen_at timestamptz, \
                 ratio float8, \
                 label varchar(255), \
                 note text, \
                 meta jsonb); \
             ALTER TABLE {SCHEMA}.widgets ENABLE ROW LEVEL SECURITY; \
             ALTER TABLE {SCHEMA}.widgets FORCE ROW LEVEL SECURITY; \
             CREATE POLICY open ON {SCHEMA}.widgets USING (true) WITH CHECK (true); \
             GRANT USAGE ON SCHEMA {SCHEMA} TO authenticated; \
             GRANT SELECT, INSERT, UPDATE, DELETE ON {SCHEMA}.widgets TO authenticated;"
        ))
        .await
        .inspect_err(|e| eprintln!("[rest_typed_columns_http] setup failed: {e}"))
        .expect("ephemeral setup");
}

/// The whole matrix in one test: the ephemeral schema is shared, and ordering
/// (insert → filter → update → delete) is inherent to the scenario.
#[tokio::test]
async fn typed_columns_round_trip_over_http() {
    let Some(url) = database_url() else {
        eprintln!("[rest_typed_columns_http] DATABASE_URL unset — skipping");
        return;
    };
    // Surface the handler's own `tracing::error!` (the real Postgres error)
    // instead of a bare 500 when something fails.
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();

    let mut setup_cfg = Config::new();
    setup_cfg.url = Some(url.clone());
    let setup_pool = setup_cfg
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("setup pool");
    let setup = setup_pool.get().await.expect("setup conn");

    if !role_authenticated_present(&setup).await {
        eprintln!("[rest_typed_columns_http] `authenticated` role absent — skipping");
        return;
    }
    seed_schema(&setup).await;

    let mut exec_cfg = Config::new();
    exec_cfg.url = Some(url);
    let exec_pool = exec_cfg
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("exec pool");
    let executor: Arc<dyn SqlExecutor> = Arc::new(PgRest::new(exec_pool));
    let router = RestCompiler::compile(&model(), executor);

    let base = format!("/{SCHEMA}/widgets");

    assert_typed_inserts(&router, &base).await;
    assert_typed_filters(&router, &base).await;
    assert_typed_patch(&router, &base, &setup).await;
    assert_typed_delete(&router, &base, &setup).await;

    setup
        .batch_execute(&format!("DROP SCHEMA {SCHEMA} CASCADE;"))
        .await
        .expect("cleanup");
}

/// POST every typed column from native JSON types.
///
/// `qty`, `big`, and `ratio` arrive as JSON *numbers* and `active` as a JSON
/// *bool* — the case the old `json_bind` bound as `QueryParam::Json`, which the
/// hand-rolled `placeholder_for` then cast `$n::integer`, producing
/// `cannot cast type jsonb to integer`.
async fn assert_typed_inserts(router: &axum::Router, base: &str) {
    let (status, body) = send(
        router,
        "POST",
        base,
        Some(json!({
            "id": ROW_A,
            "qty": 5,
            "big": 9_000_000_000_i64,
            "price": "19.99",
            "active": true,
            "received_on": "2026-07-01",
            "seen_at": "2026-07-01T12:30:00Z",
            "ratio": 1.5,
            "label": "widget-a",
            "note": "tenant-a",
            "meta": {"k": "v"}
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "typed INSERT failed: {body}");

    // Values must land as their real column types, NOT JSON-quoted strings.
    assert_eq!(body["qty"], json!(5), "int4 must be a number, not a string");
    assert_eq!(body["active"], json!(true), "bool must be a real boolean");
    assert_eq!(
        body["note"],
        json!("tenant-a"),
        "a text value must round-trip unquoted — a blanket ::jsonb cast \
         silently yields \"\\\"tenant-a\\\"\" and breaks RLS WITH CHECK"
    );
    assert_eq!(body["received_on"], json!("2026-07-01"));

    // A second row, so filters below prove selectivity rather than
    // returning everything.
    let (status, _) = send(
        router,
        "POST",
        base,
        Some(json!({
            "id": ROW_B, "qty": 50, "big": 1, "price": "1.00", "active": false,
            "received_on": "2020-01-01", "seen_at": "2020-01-01T00:00:00Z",
            "ratio": 0.5, "label": "widget-b", "note": "other", "meta": {}
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
}

/// GET filtering on each typed column.
///
/// This is the path the shipped fix never reached: the list handler called the
/// uncast `render_where`, so each of these 400/500'd before p17.
async fn assert_typed_filters(router: &axum::Router, base: &str) {
    for (query, expect_label) in [
        (format!("id=eq.{ROW_A}"), "widget-a"),
        ("qty=eq.5".to_owned(), "widget-a"),
        ("big=gt.100".to_owned(), "widget-a"),
        // Selective on BOTH sides: row B is 1.00, so `gt.10` isolates row A.
        // (`lt.20` would match both — numeric comparison is exercised either
        // way, but a filter that matches everything proves nothing.)
        ("price=gt.10".to_owned(), "widget-a"),
        ("active=eq.true".to_owned(), "widget-a"),
        ("received_on=eq.2026-07-01".to_owned(), "widget-a"),
        ("seen_at=gt.2025-01-01T00:00:00Z".to_owned(), "widget-a"),
        ("ratio=gt.1.0".to_owned(), "widget-a"),
        ("label=eq.widget-a".to_owned(), "widget-a"),
    ] {
        let (status, body) = send(router, "GET", &format!("{base}?{query}"), None).await;
        assert_eq!(status, StatusCode::OK, "GET ?{query} failed: {body}");
        let rows = body.as_array().expect("array");
        assert_eq!(rows.len(), 1, "GET ?{query} should match exactly one row");
        assert_eq!(rows[0]["label"], json!(expect_label), "GET ?{query}");
    }

    // An `in.(…)` filter binds an ARRAY — it must render `$n::integer[]`, a
    // case the hand-rolled placeholder never handled at all.
    let (status, body) = send(router, "GET", &format!("{base}?qty=in.(5,50)"), None).await;
    assert_eq!(status, StatusCode::OK, "array-bound filter failed: {body}");
    assert_eq!(body.as_array().expect("array").len(), 2);
}

/// PATCH filtering on a typed column while SETting others, then verify the
/// written values server-side through a separate connection.
///
/// The second connection is the point: the handler's own `RETURNING` reads
/// inside its RLS transaction, so it reports success even when that transaction
/// is later rolled back. Only an independent read proves the write persisted.
async fn assert_typed_patch(router: &axum::Router, base: &str, setup: &deadpool_postgres::Object) {
    let (status, body) = send(
        router,
        "PATCH",
        &format!("{base}?id=eq.{ROW_A}"),
        Some(json!({"qty": 7, "active": false, "price": "25.50"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "typed PATCH failed: {body}");
    let rows = body.as_array().expect("array");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["qty"], json!(7));
    assert_eq!(rows[0]["active"], json!(false));

    // Verify server-side, bypassing the handler, that real column types were
    // written — a string "7" in an int4 column cannot happen, but a JSON-quoted
    // numeric or a stringified bool would show up here.
    // `$1::text::uuid`, not `$1::uuid`, and not a bound `uuid::Uuid`:
    // `tokio-postgres` here is built with `with-serde_json-1` only, so `Uuid`
    // has no `ToSql` impl — and a bare `$1::uuid` makes Postgres infer the
    // parameter as `uuid`, which the driver then rejects against a bound `&str`
    // (`WrongType { postgres: Uuid, rust: "&str" }`). Routing through `text` is
    // the same fix `mutation_placeholder` applies to the production path.
    let row = setup
        .query_one(
            &format!(
                "SELECT qty, active, price::text FROM {SCHEMA}.widgets \
                 WHERE id = $1::text::uuid"
            ),
            &[&ROW_A],
        )
        .await
        .expect("verify row");
    let qty: i32 = row.get("qty");
    let active: bool = row.get("active");
    let price: String = row.get("price");
    assert_eq!(qty, 7);
    assert!(!active);
    assert_eq!(price, "25.50");
}

/// DELETE filtering on a `uuid` primary key, verified by an independent count.
async fn assert_typed_delete(router: &axum::Router, base: &str, setup: &deadpool_postgres::Object) {
    let (status, _) = send(router, "DELETE", &format!("{base}?id=eq.{ROW_A}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "typed DELETE failed");

    let remaining: i64 = setup
        .query_one(&format!("SELECT count(*) FROM {SCHEMA}.widgets"), &[])
        .await
        .expect("count")
        .get(0);
    assert_eq!(remaining, 1, "only the uuid-filtered row should be deleted");
}
