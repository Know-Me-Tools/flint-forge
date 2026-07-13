//! p3-c010 gate test: reflection router merges into the gateway Router without
//! producing 404 on reflection-compiled routes.
//!
//! This test only proves mounting — that the reflection router's routes are
//! present in the merged gateway `Router` and don't shadow/get-shadowed-by
//! `/healthz`. CRUD handler bodies are fully implemented (see
//! `fdb-reflection/src/compilers/rest/mod.rs`); a request to `/public/widget`
//! here hits a `PgRest` executor over a `connect_lazy`'d (never-dialed) pool,
//! so the handler returns a `500` connection error — it does not panic and it
//! is not a `404`. Full RLS-isolation behavior is covered by the
//! `DATABASE_URL`-gated two-tenant test in `fdb-postgres`/`fdb-reflection`
//! integration tests (p16-c001), not here.

#![forbid(unsafe_code)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use fdb_postgres::PgRest;
use fdb_reflection::{
    compilers::rest::RestCompiler,
    model::{Column, DatabaseModel, Table},
};
use std::sync::Arc;
use tower::ServiceExt;

fn fixture_model() -> DatabaseModel {
    DatabaseModel {
        tables: vec![Table {
            schema: "public".into(),
            name: "widget".into(),
            columns: vec![Column {
                name: "id".into(),
                pg_type: "int4".into(),
                nullable: false,
                default: None,
            }],
            pk: vec!["id".into()],
            fk: vec![],
            rls_enabled: true,
            vault_key: None,
        }],
        functions: vec![],
        views: vec![],
        version: 1,
    }
}

async fn healthz() -> &'static str {
    "ok"
}

#[tokio::test]
async fn reflection_router_mounted_not_404() {
    let model = fixture_model();

    // A deadpool_postgres pool config doesn't dial until the first `.get()` —
    // this test only asserts the route is mounted, not that it can reach a DB.
    let mut cfg = deadpool_postgres::Config::new();
    cfg.url = Some("postgres://localhost/flint".to_owned());
    let pool = cfg
        .create_pool(
            Some(deadpool_postgres::Runtime::Tokio1),
            tokio_postgres::NoTls,
        )
        .expect("lazy pool create should not dial");
    let executor: Arc<dyn fdb_ports::SqlExecutor> = Arc::new(PgRest::new(pool));

    let reflection_router = RestCompiler::compile(&model, executor);

    // Mimic the gateway composition pattern from main.rs:
    // build the gateway routes as Router<()> then .merge(reflection_router).
    let app = Router::new()
        .route("/healthz", get(healthz))
        .merge(reflection_router);

    // healthz is reachable (proves the gateway side of the merge).
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request");
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "/healthz must remain reachable after merge"
    );

    // The reflection route /public/widget exists (GET). No `RlsContext`
    // extension is inserted (this test merges the reflection router directly,
    // without the `require_rls` auth middleware layer), so `handle_list`'s
    // `Extension<RlsContext>` extraction fails with a `500` before any SQL
    // runs — still a resolved route, not a `404`, which is exactly what this
    // test exists to prove (mounting), not handler behavior.
    let widget_request = Request::builder()
        .uri("/public/widget")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(widget_request).await.expect("request");
    assert_ne!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "/public/widget must be mounted (got 404 = mount broken)"
    );
}
