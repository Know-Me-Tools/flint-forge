//! p16-c001 gate test: REST/RPC tenant isolation under RLS.
//!
//! `DATABASE_URL`-gated; also requires the `authenticated` role (provisioned
//! by the `ext-flint-auth` pgrx extension, present in the CI Postgres image
//! built from `images/postgres18`) — skips cleanly if either is absent, so
//! `cargo test --workspace` never requires a database.
//!
//! Seeds a two-tenant table with an RLS policy keyed on
//! `request.jwt.claims->>'tenant_id'`, compiles it through the real
//! `RestCompiler`, and drives requests through the actual Axum router — the
//! same handler code path production traffic uses — with `Extension<RlsContext>`
//! inserted directly into each request (bypassing JWT verification, which is
//! `forge-identity`'s own separately-tested concern). Proves a tenant-A
//! context cannot read or mutate tenant-B's rows over GET/POST/PATCH/DELETE,
//! and CAN read/mutate its own — the two-sided assertion that guards against
//! an overly strict policy producing a false-positive "isolated" result.

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
};
use forge_identity::RlsContext;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio_postgres::NoTls;
use tower::ServiceExt;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty())
}

fn rls_for_tenant(tenant: &str) -> RlsContext {
    RlsContext {
        role: "authenticated".into(),
        claims_json: format!(r#"{{"tenant_id":"{tenant}"}}"#),
        raw_bearer: "test-token".into(),
        keto_subject: format!("user-{tenant}"),
        vault_key_id: None,
    }
}

fn model() -> DatabaseModel {
    DatabaseModel {
        tables: vec![Table {
            schema: "p16c001_it".into(),
            name: "orders".into(),
            columns: vec![
                Column {
                    name: "id".into(),
                    pg_type: "text".into(),
                    nullable: false,
                    default: None,
                },
                Column {
                    name: "tenant_id".into(),
                    pg_type: "text".into(),
                    nullable: false,
                    default: None,
                },
                Column {
                    name: "note".into(),
                    pg_type: "text".into(),
                    nullable: true,
                    default: None,
                },
            ],
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

/// Build and send a request through the compiled router with `rls` inserted
/// directly into request extensions — exactly what `require_rls` middleware
/// does on the real gateway, minus JWT verification itself.
async fn send(
    router: &axum::Router,
    method: &str,
    uri: &str,
    rls: RlsContext,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    let body_bytes = body.map_or_else(String::new, |v| v.to_string());
    let mut req = builder.body(Body::from(body_bytes)).expect("build request");
    req.extensions_mut().insert(rls);

    let resp = router.clone().oneshot(req).await.expect("request");
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("read body");
    let json: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, json)
}

/// Returns `false` (test should skip) when `authenticated` doesn't exist —
/// `ext-flint-auth` not installed in this Postgres.
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

/// Ephemeral schema/table/policy/seed for the isolation scenario.
///
/// `id` is deliberately `text`, not `int4`/`uuid`: two separate, broader bugs
/// were discovered writing this test — (1) any WHERE-clause filter against a
/// non-text column fails at the driver's parameter-type-inference step, and
/// (2) inserting a JSON-bound value into a non-text column fails to coerce
/// server-side. Both are tracked as their own follow-up work, independent of
/// this change's actual scope (RLS enforcement). Using `text` throughout
/// keeps this test a clean, unblocked proof of tenant isolation.
async fn seed_schema(setup: &deadpool_postgres::Object) {
    setup
        .batch_execute(
            "DROP SCHEMA IF EXISTS p16c001_it CASCADE; \
             CREATE SCHEMA p16c001_it; \
             CREATE TABLE p16c001_it.orders (id text PRIMARY KEY, tenant_id text NOT NULL, note text); \
             ALTER TABLE p16c001_it.orders ENABLE ROW LEVEL SECURITY; \
             ALTER TABLE p16c001_it.orders FORCE ROW LEVEL SECURITY; \
             CREATE POLICY tenant_isolation ON p16c001_it.orders \
               USING (tenant_id = current_setting('request.jwt.claims', true)::jsonb->>'tenant_id') \
               WITH CHECK (tenant_id = current_setting('request.jwt.claims', true)::jsonb->>'tenant_id'); \
             GRANT USAGE ON SCHEMA p16c001_it TO authenticated; \
             GRANT SELECT, INSERT, UPDATE, DELETE ON p16c001_it.orders TO authenticated; \
             INSERT INTO p16c001_it.orders VALUES ('1', 'tenant-a', 'a-secret'), ('2', 'tenant-b', 'b-secret');",
        )
        .await
        .inspect_err(|e| eprintln!("[rest_rls_isolation] setup failed: {e}"))
        .expect("ephemeral setup");
}

/// Each tenant's unfiltered list sees exactly its own row; a direct filter
/// for the other tenant's row returns nothing. Filters here target only text
/// columns (`tenant_id`) — a separate, broader bug (flagged and tracked
/// independently of this change) means filtering an `int4` column like `id`
/// fails at the driver level regardless of RLS; scoping this test to text
/// columns keeps it a clean proof of tenant isolation specifically.
async fn assert_list_isolation(router: &axum::Router, tenant_a: &RlsContext, tenant_b: &RlsContext) {
    let (status, body) = send(router, "GET", "/p16c001_it/orders", tenant_a.clone(), None).await;
    assert_eq!(status, StatusCode::OK);
    let rows = body.as_array().expect("array");
    assert_eq!(rows.len(), 1, "tenant A sees exactly its own row, not tenant B's");
    assert_eq!(rows[0]["tenant_id"], "tenant-a");
    assert_eq!(rows[0]["note"], "a-secret");

    let (status, body) = send(router, "GET", "/p16c001_it/orders", tenant_b.clone(), None).await;
    assert_eq!(status, StatusCode::OK);
    let rows = body.as_array().expect("array");
    assert_eq!(rows.len(), 1, "tenant B sees exactly its own row, not tenant A's");
    assert_eq!(rows[0]["tenant_id"], "tenant-b");

    let (_, body) = send(
        router,
        "GET",
        "/p16c001_it/orders?tenant_id=eq.tenant-b",
        tenant_a.clone(),
        None,
    )
    .await;
    assert_eq!(
        body.as_array().expect("array").len(),
        0,
        "tenant A cannot read tenant B's row by a direct tenant_id filter"
    );
}

/// Tenant A's PATCH/DELETE targeting tenant B's row match zero rows; tenant
/// B's row is verified unchanged/present via a direct DB read.
async fn assert_mutation_isolation(
    router: &axum::Router,
    setup: &deadpool_postgres::Object,
    tenant_a: &RlsContext,
) {
    let (status, _) = send(
        router,
        "PATCH",
        "/p16c001_it/orders?tenant_id=eq.tenant-b",
        tenant_a.clone(),
        Some(json!({"note": "hacked"})),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NO_CONTENT,
        "0 rows matched — tenant B's row must be untouched"
    );
    let note: String = setup
        .query_one("SELECT note FROM p16c001_it.orders WHERE id = '2'", &[])
        .await
        .expect("verify")
        .get(0);
    assert_eq!(
        note, "b-secret",
        "tenant A's PATCH must not have reached tenant B's row"
    );

    let (status, _) = send(
        router,
        "DELETE",
        "/p16c001_it/orders?tenant_id=eq.tenant-b",
        tenant_a.clone(),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let count: i64 = setup
        .query_one("SELECT count(*) FROM p16c001_it.orders WHERE id = '2'", &[])
        .await
        .expect("verify")
        .get(0);
    assert_eq!(
        count, 1,
        "tenant B's row must still exist — tenant A's DELETE must not have reached it"
    );
}

/// Same-tenant round trip (guards against a false-positive "isolated" result
/// from an overly strict policy), plus the `WITH CHECK` spoofing guard.
async fn assert_same_tenant_and_check_constraint(router: &axum::Router, tenant_a: RlsContext) {
    let (status, _) = send(
        router,
        "POST",
        "/p16c001_it/orders",
        tenant_a.clone(),
        Some(json!({"id": "3", "tenant_id": "tenant-a", "note": "new"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "tenant A can insert its own row");

    let (_, body) = send(
        router,
        "GET",
        "/p16c001_it/orders?note=eq.new",
        tenant_a.clone(),
        None,
    )
    .await;
    assert_eq!(
        body.as_array().expect("array").len(),
        1,
        "tenant A sees its own newly-inserted row"
    );

    let (status, _) = send(
        router,
        "POST",
        "/p16c001_it/orders",
        tenant_a,
        Some(json!({"id": "4", "tenant_id": "tenant-b", "note": "spoofed"})),
    )
    .await;
    assert_ne!(
        status,
        StatusCode::CREATED,
        "tenant A must not be able to insert a row tagged as tenant B"
    );
}

#[tokio::test]
async fn rest_crud_enforces_tenant_isolation() {
    // Surfaces the real `tracing::error!` from a failing handler (e.g. the
    // actual Postgres error) instead of just the generic 500 status — set
    // `RUST_LOG` to raise the level when debugging a failure.
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
    let Some(url) = database_url() else {
        eprintln!("[rest_rls_isolation] DATABASE_URL unset — skipping");
        return;
    };

    let mut cfg = Config::new();
    cfg.url = Some(url);
    let setup_pool = cfg
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("setup pool");
    let setup = setup_pool.get().await.expect("conn");

    if !role_authenticated_present(&setup).await {
        eprintln!(
            "[rest_rls_isolation] 'authenticated' role not present (ext-flint-auth not \
             installed) — skipping"
        );
        return;
    }
    seed_schema(&setup).await;

    let mut exec_cfg = Config::new();
    exec_cfg.url = std::env::var("DATABASE_URL").ok();
    let exec_pool = exec_cfg
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("exec pool");
    let executor: Arc<dyn SqlExecutor> = Arc::new(PgRest::new(exec_pool));
    let router = RestCompiler::compile(&model(), executor);

    let tenant_a = rls_for_tenant("tenant-a");
    let tenant_b = rls_for_tenant("tenant-b");

    assert_list_isolation(&router, &tenant_a, &tenant_b).await;
    assert_mutation_isolation(&router, &setup, &tenant_a).await;
    assert_same_tenant_and_check_constraint(&router, tenant_a).await;

    setup
        .batch_execute("DROP SCHEMA p16c001_it CASCADE;")
        .await
        .expect("cleanup");
}
