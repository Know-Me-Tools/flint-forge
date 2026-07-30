//! p17-c006 phase-boundary end-to-end tests (FFS-001 §9).
//!
//! 1. **Two-tenant RLS isolation on a PROVISIONED table** — "the test that
//!    matters most": one table created via `/plan`+`/apply` with the real
//!    `service_role` key, then tenant A inserts and tenant B must see
//!    nothing, through the exact GUC mechanism production uses
//!    (`SET LOCAL ROLE authenticated` + `request.jwt.claims`).
//! 2. **OpenAPI hot-swap** — the provisioned table appears in the compiled
//!    OpenAPI document WITHOUT restart. Valid on v1's `restartRequired:true`
//!    contract because `openapi_handler` loads `state_manager.current()` per
//!    request (`src/handlers.rs:36–38`); only REST *routes* are
//!    restart-bound (FFS-001 D8).
//! 3. **Key-rotation revocation** — after an operator rotation (re-running
//!    `generate-keys.mjs` and refreshing the served JWKS), the OLD
//!    `service_role` key must fail with 401. Gated on
//!    `FLINT_OLD_SERVICE_ROLE_KEY`, which only exists post-rotation, because
//!    the test must not rotate the operator's real keys itself.

#![allow(clippy::expect_used)]

use axum::http::StatusCode;
use fdb_gateway_test_support::*;
use tokio_postgres::NoTls;
use tower::ServiceExt;

#[path = "support/schema_routes_support.rs"]
mod fdb_gateway_test_support;

/// Open a connection and run one statement batch under a tenant's RLS
/// context — the same `SET LOCAL` statements `PgBackend::acquire` issues.
async fn as_tenant(
    url: &str,
    tenant: &str,
    sql: &str,
) -> Result<Vec<tokio_postgres::Row>, tokio_postgres::Error> {
    let (client, conn) = tokio_postgres::connect(url, NoTls).await.expect("connect");
    tokio::spawn(async move {
        let _ = conn.await;
    });
    client.batch_execute("BEGIN").await?;
    client.batch_execute("SET LOCAL ROLE authenticated").await?;
    client
        .execute(
            "SELECT set_config('request.jwt.claims', $1, true)",
            &[&format!(
                r#"{{"tenant_id":"{tenant}","role":"authenticated"}}"#
            )],
        )
        .await?;
    let rows = client.query(sql, &[]).await;
    client.batch_execute("COMMIT").await?;
    rows
}

#[tokio::test]
async fn provisioned_table_isolates_two_tenants() {
    let Some(env) = TestEnv::with_keys().await else {
        return;
    };
    let ns = "p17c006_iso";
    env.reset_namespace(ns).await;
    // Per runbook §14: authenticated needs USAGE on operator namespaces.
    let (admin, aconn) = tokio_postgres::connect(&env.db_url, NoTls)
        .await
        .expect("admin");
    tokio::spawn(async move {
        let _ = aconn.await;
    });
    admin
        .batch_execute(&format!("GRANT USAGE ON SCHEMA {ns} TO authenticated;"))
        .await
        .expect("grant usage");

    // Provision through the REAL API with the REAL key.
    let router = env.router_enabled(&[ns]);
    let response = router
        .clone()
        .oneshot(request(
            "POST",
            "/schema/v1/plan",
            Some(&env.service_key),
            spec_body(ns, "notes").as_bytes(),
        ))
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK);
    let hash = body_json(response).await["planHash"]
        .as_str()
        .expect("hash")
        .to_owned();
    let response = router
        .oneshot(request(
            "POST",
            "/schema/v1/apply",
            Some(&env.service_key),
            format!(r#"{{"planHash":"{hash}"}}"#).as_bytes(),
        ))
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK, "apply must succeed");

    // Tenant A inserts a row under its own RLS context.
    as_tenant(
        &env.db_url,
        "tenant-a",
        &format!(
            "INSERT INTO {ns}.notes (id, payload, tenant_id) \
             VALUES ('n1', '{{}}', 'tenant-a') RETURNING id"
        ),
    )
    .await
    .expect("tenant A insert must pass its own policy");

    // Tenant B sees NOTHING — the assertion the whole feature exists for.
    let rows = as_tenant(
        &env.db_url,
        "tenant-b",
        &format!("SELECT id FROM {ns}.notes"),
    )
    .await
    .expect("tenant B select runs");
    assert!(
        rows.is_empty(),
        "TENANT ISOLATION FAILED: tenant B can see tenant A's rows"
    );

    // Tenant A sees its own row (two-sided: an overly strict policy would
    // produce a false-positive "isolated" result).
    let rows = as_tenant(
        &env.db_url,
        "tenant-a",
        &format!("SELECT id FROM {ns}.notes"),
    )
    .await
    .expect("tenant A select runs");
    assert_eq!(rows.len(), 1, "tenant A must see its own row");

    // Tenant B cannot forge tenant A's tenant_id on insert (WITH CHECK).
    let forged = as_tenant(
        &env.db_url,
        "tenant-b",
        &format!(
            "INSERT INTO {ns}.notes (id, payload, tenant_id) \
             VALUES ('n2', '{{}}', 'tenant-a') RETURNING id"
        ),
    )
    .await;
    assert!(
        forged.is_err(),
        "cross-tenant INSERT must violate the WITH CHECK policy"
    );

    env.reset_namespace(ns).await;
}

#[tokio::test]
async fn provisioned_table_appears_in_openapi_without_restart() {
    let Some(env) = TestEnv::with_keys().await else {
        return;
    };
    let ns = "p17c006_oas";
    env.reset_namespace(ns).await;

    // Start the reflection listener (bootstrap does this at startup) and
    // subscribe to the hot-swap watch channel.
    let mut versions = env.state_manager.subscribe_version();
    let _listener = std::sync::Arc::clone(&env.state_manager).start_listener();

    let router = env.router_enabled(&[ns]);
    let response = router
        .clone()
        .oneshot(request(
            "POST",
            "/schema/v1/plan",
            Some(&env.service_key),
            spec_body(ns, "hotswap").as_bytes(),
        ))
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK);
    let hash = body_json(response).await["planHash"]
        .as_str()
        .expect("hash")
        .to_owned();
    let response = router
        .oneshot(request(
            "POST",
            "/schema/v1/apply",
            Some(&env.service_key),
            format!(r#"{{"planHash":"{hash}"}}"#).as_bytes(),
        ))
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK, "apply must succeed");

    // Wait (bounded) for the event-trigger → NOTIFY → recompile hot-swap.
    let waited = tokio::time::timeout(std::time::Duration::from_secs(30), versions.changed()).await;
    assert!(
        waited.is_ok(),
        "reflection hot-swap did not fire within 30s of apply"
    );

    // The same document `/openapi.json` serves per request
    // (openapi_handler → state_manager.current().openapi_doc).
    let doc = env.state_manager.current().openapi_doc.clone();
    let doc_text = doc.to_string();
    assert!(
        doc_text.contains("hotswap"),
        "provisioned table must appear in the hot-swapped OpenAPI document without restart"
    );

    env.reset_namespace(ns).await;
}

/// Rotation is the revocation path for a 10-year key (FFS-001 §9). This test
/// runs only AFTER an operator rotation: export the pre-rotation token as
/// `FLINT_OLD_SERVICE_ROLE_KEY` (with the refreshed JWKS served at
/// `FLINT_GATE_JWKS_URL`), and the old key must now die with 401 while the
/// new `FLINT_SERVICE_ROLE_KEY` still passes.
///
/// Manual procedure (record the transcript in the phase verification):
/// 1. `cp .env.keys .env.keys.pre-rotation`
/// 2. re-run `node infra/scripts/generate-keys.mjs` (ROTATES the keypair)
/// 3. restart/refresh whatever serves `infra/keys/jwks.json`
/// 4. `FLINT_OLD_SERVICE_ROLE_KEY=$(source .env.keys.pre-rotation; echo $FLINT_SERVICE_ROLE_KEY)`
///    plus the new `.env.keys` values → run this test.
#[tokio::test]
async fn rotated_out_service_role_key_is_refused() {
    let Some(env) = TestEnv::with_keys().await else {
        return;
    };
    let Some(old_key) = env_nonempty("FLINT_OLD_SERVICE_ROLE_KEY") else {
        eprintln!("skipping rotation test: FLINT_OLD_SERVICE_ROLE_KEY not set (run after a real rotation)");
        return;
    };

    let router = env.router_enabled(&["p17c006_rot"]);
    // New key passes the gate (reaches namespace validation, not 401/403).
    let response = router
        .clone()
        .oneshot(request(
            "POST",
            "/schema/v1/plan",
            Some(&env.service_key),
            b"{}",
        ))
        .await
        .expect("infallible");
    assert_ne!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "new key must verify"
    );

    // Old key dies: its kid is gone from the served JWKS, and
    // refetch-on-unknown-kid (p16-c005) makes that authoritative promptly.
    let response = router
        .oneshot(request("POST", "/schema/v1/plan", Some(&old_key), b"{}"))
        .await
        .expect("infallible");
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "the pre-rotation key must be refused once the JWKS no longer carries its kid"
    );
}
