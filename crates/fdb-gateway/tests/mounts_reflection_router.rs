//! p3-c010 gate test: reflection router merges into the gateway Router without
//! producing 404 on reflection-compiled routes.
//!
//! Full CRUD body testing (request actually reaching a handler body, not the
//! axum route table) lands in p3-c015 once `handle_list` etc. are implemented
//! (currently `todo!()`).
//!
//! This test constructs a minimal `DatabaseModel` with one table, compiles it
//! via `RestCompiler`, merges the resulting `Router<()>` into a gateway-shaped
//! router (with `/healthz`), and asserts that a request to the compiled table
//! route does NOT return 404 — proving the reflection router is mounted.

#![forbid(unsafe_code)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use fdb_reflection::{
    compilers::rest::RestCompiler,
    model::{Column, DatabaseModel, Table},
};
use sqlx::PgPool;
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

    // connect_lazy builds the pool object without opening a connection.
    // The handler bodies are `todo!()` so no query is actually executed; we
    // only assert the route exists (not a 404 from a missing mount).
    let pool =
        PgPool::connect_lazy("postgres://localhost/flint").expect("connect_lazy should not dial");

    let reflection_router = RestCompiler::compile(&model, pool);

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

    // The reflection route /public/widget exists (GET). Since handle_list is
    // still todo!(), the handler panics — but the route itself resolves (not
    // 404). We spawn the request in a separate task so the panic is isolated:
    // a matched route panics (expected), an unmatched route returns 404.
    let widget_request = Request::builder()
        .uri("/public/widget")
        .body(Body::empty())
        .unwrap();
    let app_for_widget = app.clone();

    let join = tokio::spawn(async move { app_for_widget.oneshot(widget_request).await });

    if let Ok(Ok(resp)) = join.await {
        // Handler returned a response (shouldn't happen yet with todo!(),
        // but if it does, it must NOT be 404).
        assert_ne!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "/public/widget must be mounted (got 404 = mount broken)"
        );
    } else {
        // Handler panicked (todo!()) or errored — proves the route was
        // matched. A 404 would have returned a response, not panicked.
    }
}
