//! Regression test for the REST router path-extraction mismatch:
//! `endpoint_generation::generate()` bakes `schema`/`table` into each route's
//! path as literal segments (zero axum path captures), but the REST handlers
//! used to declare `Path<(String, String)>`, which axum can only satisfy when
//! the URL template declares two captures. Every request hit that mismatch and
//! failed with `500` before ever reaching handler logic — confirmed via
//! `PathDeserializationError: Wrong number of path arguments for \`Path\`.
//! Expected 2 but got 0`.
//!
//! DATABASE_URL-gated: runs against a real ephemeral Postgres table/schema
//! when `DATABASE_URL` is set, skips cleanly otherwise (same convention as
//! `fdb-postgres/tests/pgrest_live_pg.rs`).
#![allow(clippy::expect_used)]

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    Router,
};
use fdb_postgres::PgRest;
use fdb_reflection::{
    compilers::rest::RestCompiler,
    model::{Column, DatabaseModel, FnMeta, Table},
};
use forge_identity::RlsContext;
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty())
}

fn test_rls() -> RlsContext {
    RlsContext {
        role: "authenticated".into(),
        claims_json: "{}".into(),
        raw_bearer: "t".into(),
        keto_subject: "test".into(),
        vault_key_id: None,
    }
}

fn fixture_model() -> DatabaseModel {
    DatabaseModel {
        tables: vec![Table {
            schema: "restrouter_it".into(),
            name: "widget".into(),
            columns: vec![
                Column {
                    name: "id".into(),
                    pg_type: "int4".into(),
                    nullable: false,
                    default: Some("nextval".into()),
                },
                Column {
                    name: "status".into(),
                    pg_type: "text".into(),
                    nullable: true,
                    default: None,
                },
            ],
            pk: vec!["id".into()],
            fk: vec![],
            rls_enabled: false,
            vault_key: None,
        }],
        functions: vec![FnMeta {
            schema: "restrouter_it".into(),
            name: "echo_one".into(),
            args: vec![],
            return_type: "record".into(),
            security_definer: false,
        }],
        views: vec![],
        version: 1,
    }
}

/// GET /restrouter_it/widget — the exact reported repro (list route, zero
/// path captures). Must not be 500.
async fn assert_list_ok(app: &Router) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/restrouter_it/widget")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("list request");
    assert_ne!(
        resp.status(),
        StatusCode::INTERNAL_SERVER_ERROR,
        "GET list route must not 500 on path extraction"
    );
    assert_eq!(resp.status(), StatusCode::OK);
}

/// POST /restrouter_it/widget — insert route.
async fn assert_insert_created(app: &Router, rls: &RlsContext) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/restrouter_it/widget")
                .header(header::CONTENT_TYPE, "application/json")
                .extension(rls.clone())
                .body(Body::from(r#"{"status":"new"}"#))
                .unwrap(),
        )
        .await
        .expect("insert request");
    assert_ne!(
        resp.status(),
        StatusCode::INTERNAL_SERVER_ERROR,
        "POST insert route must not 500 on path extraction"
    );
    assert_eq!(resp.status(), StatusCode::CREATED);
}

/// PATCH /restrouter_it/widget?status=eq.new — update route.
async fn assert_update_ok(app: &Router, rls: &RlsContext) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/restrouter_it/widget?status=eq.new")
                .header(header::CONTENT_TYPE, "application/json")
                .extension(rls.clone())
                .body(Body::from(r#"{"status":"updated"}"#))
                .unwrap(),
        )
        .await
        .expect("update request");
    assert_ne!(
        resp.status(),
        StatusCode::INTERNAL_SERVER_ERROR,
        "PATCH update route must not 500 on path extraction"
    );
}

/// DELETE /restrouter_it/widget?status=eq.updated — delete route.
async fn assert_delete_no_content(app: &Router, rls: &RlsContext) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/restrouter_it/widget?status=eq.updated")
                .extension(rls.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("delete request");
    assert_ne!(
        resp.status(),
        StatusCode::INTERNAL_SERVER_ERROR,
        "DELETE route must not 500 on path extraction"
    );
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

/// POST /rpc/restrouter_it/echo_one — RPC route (same bug class).
async fn assert_rpc_ok(app: &Router) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/rpc/restrouter_it/echo_one")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .expect("rpc request");
    assert_ne!(
        resp.status(),
        StatusCode::INTERNAL_SERVER_ERROR,
        "RPC route must not 500 on path extraction"
    );
}

/// Drives GET/POST/PATCH/DELETE table routes and the RPC route through the
/// compiled router against a real ephemeral schema, asserting none return the
/// `500` that `Path<(String, String)>` extraction failure produced.
#[tokio::test]
async fn rest_router_extracts_schema_and_table_without_path_captures() {
    let Some(url) = database_url() else {
        eprintln!("[rest_router_extraction] DATABASE_URL unset — skipping");
        return;
    };

    let pool = PgPool::connect(&url).await.expect("connect");
    sqlx::raw_sql(
        "DROP SCHEMA IF EXISTS restrouter_it CASCADE; \
         CREATE SCHEMA restrouter_it; \
         CREATE TABLE restrouter_it.widget (id serial PRIMARY KEY, status text); \
         CREATE FUNCTION restrouter_it.echo_one() RETURNS TABLE(val int, val2 int) \
             LANGUAGE sql AS $$ SELECT 1, 2 $$;",
    )
    .execute(&pool)
    .await
    .expect("ephemeral setup");

    let deadpool = {
        let mut cfg = deadpool_postgres::Config::new();
        cfg.url = Some(url.clone());
        cfg.create_pool(
            Some(deadpool_postgres::Runtime::Tokio1),
            tokio_postgres::NoTls,
        )
        .expect("rest-executor pool create")
    };
    let executor: Arc<dyn fdb_ports::SqlExecutor> = Arc::new(PgRest::new(deadpool));

    let model = fixture_model();
    let app = RestCompiler::compile(&model, executor);
    let rls = test_rls();

    assert_list_ok(&app).await;
    assert_insert_created(&app, &rls).await;
    assert_update_ok(&app, &rls).await;
    assert_delete_no_content(&app, &rls).await;
    assert_rpc_ok(&app).await;

    sqlx::query("DROP SCHEMA restrouter_it CASCADE;")
        .execute(&pool)
        .await
        .expect("cleanup");
}
