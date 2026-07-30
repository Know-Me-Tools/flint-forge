//! Route tests for `/schema/v1` (p17-c004, FFS-001 §8 Phase 3 gate).
//!
//! Layered gating so each case runs in the richest environment available and
//! skips cleanly below it:
//! - `DATABASE_URL` (+ migration 0015 artifacts) — needed by every case
//!   (`SchemaApiState` holds a live `StateManager`).
//! - Real keys (`FLINT_SERVICE_ROLE_KEY`, `FLINT_ANON_KEY`, served
//!   `FLINT_GATE_JWKS_URL`, issuer/audience) — needed by the role-matrix and
//!   success-path cases, per FFS-001 §9: use the actual credentials, not
//!   synthetic tokens.
//!
//! Matrix (each its own test): 503 disabled (pinned as 503, NOT 404), 401 no
//! header, 401 bad signature, 403 anon role, 403 reserved namespace, 403
//! non-allowlisted namespace, 200 plan, 200 apply → table exists, 200 replay
//! alreadyApplied, 409 drift.

#![allow(clippy::expect_used)]

use axum::http::StatusCode;
use axum::Router;
use fdb_gateway_test_support::*;
use tower::ServiceExt;

/// Everything shared by the cases lives in a support module compiled into
/// this test binary.
#[path = "support/schema_routes_support.rs"]
mod fdb_gateway_test_support;

#[tokio::test]
async fn disabled_deployment_returns_503_not_404() {
    let Some(env) = TestEnv::database_only().await else { return };
    let router = env.router_disabled();
    for (method, uri) in [("GET", "/schema/v1/status"), ("POST", "/schema/v1/plan"), ("POST", "/schema/v1/apply")] {
        let response = router
            .clone()
            .oneshot(request(method, uri, None, b"{}"))
            .await
            .expect("infallible");
        assert_eq!(
            response.status(),
            StatusCode::SERVICE_UNAVAILABLE,
            "{method} {uri} must be 503 when disabled — a 404 would mean the route was not mounted"
        );
        let body = body_json(response).await;
        assert_eq!(body["error"], "schema provisioning is not enabled");
    }
}

#[tokio::test]
async fn missing_authorization_header_is_401() {
    let Some(env) = TestEnv::database_only().await else { return };
    let router = env.router_enabled(&["p17c004_nsx"]);
    let response = router
        .oneshot(request("POST", "/schema/v1/plan", None, b"{}"))
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn garbage_bearer_is_401() {
    let Some(env) = TestEnv::database_only().await else { return };
    let router = env.router_enabled(&["p17c004_nsx"]);
    let response = router
        .oneshot(request("POST", "/schema/v1/plan", Some("not-a-jwt"), b"{}"))
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn anon_key_is_403() {
    let Some(env) = TestEnv::with_keys().await else { return };
    let Some(anon) = env.anon_key.clone() else {
        eprintln!("skipping: FLINT_ANON_KEY not set");
        return;
    };
    let router = env.router_enabled(&["p17c004_nsx"]);
    let response = router
        .oneshot(request("POST", "/schema/v1/plan", Some(&anon), b"{}"))
        .await
        .expect("infallible");
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "the publishable anon key must never reach a provisioning route"
    );
}

#[tokio::test]
async fn reserved_namespace_is_403_even_when_allowlisted() {
    let Some(env) = TestEnv::with_keys().await else { return };
    // Operator misconfiguration: reserved names must lose to the hard refusal.
    let router = env.router_enabled(&["flint_meta"]);
    let response = router
        .oneshot(request(
            "POST",
            "/schema/v1/plan",
            Some(&env.service_key),
            spec_body("flint_meta", "t").as_bytes(),
        ))
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = body_json(response).await;
    assert!(body["error"].as_str().expect("error string").contains("reserved"));
}

#[tokio::test]
async fn non_allowlisted_namespace_is_403() {
    let Some(env) = TestEnv::with_keys().await else { return };
    let router = env.router_enabled(&["p17c004_other"]);
    let response = router
        .oneshot(request(
            "POST",
            "/schema/v1/plan",
            Some(&env.service_key),
            spec_body("p17c004_notlisted", "t").as_bytes(),
        ))
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn plan_apply_replay_and_drift_full_cycle() {
    let Some(env) = TestEnv::with_keys().await else { return };
    let ns = "p17c004_cycle";
    env.reset_namespace(ns).await;
    let router: Router = env.router_enabled(&[ns]);

    // 200 plan
    let response = router
        .clone()
        .oneshot(request(
            "POST",
            "/schema/v1/plan",
            Some(&env.service_key),
            spec_body(ns, "watch").as_bytes(),
        ))
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK, "plan must succeed");
    let plan = body_json(response).await;
    let hash = plan["planHash"].as_str().expect("planHash").to_owned();
    assert_eq!(plan["noop"], false);
    assert!(plan["ddl"].as_str().expect("ddl").contains("FORCE ROW LEVEL SECURITY"));

    // 200 apply → table exists on a fresh connection
    let apply_body = format!(r#"{{"planHash":"{hash}"}}"#);
    let response = router
        .clone()
        .oneshot(request("POST", "/schema/v1/apply", Some(&env.service_key), apply_body.as_bytes()))
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK, "apply must succeed");
    let applied = body_json(response).await;
    assert_eq!(applied["applied"], true);
    assert_eq!(applied["alreadyApplied"], false);
    assert_eq!(applied["restartRequired"], true);
    assert!(env.table_exists(ns, "watch").await, "table must exist after apply");

    // 200 replay → alreadyApplied
    let response = router
        .clone()
        .oneshot(request("POST", "/schema/v1/apply", Some(&env.service_key), apply_body.as_bytes()))
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_json(response).await["alreadyApplied"], true);

    // 409 drift: fresh plan for a second table, then mutate the namespace out
    // of band before applying.
    let response = router
        .clone()
        .oneshot(request(
            "POST",
            "/schema/v1/plan",
            Some(&env.service_key),
            spec_body(ns, "drifty").as_bytes(),
        ))
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK);
    let hash2 = body_json(response).await["planHash"].as_str().expect("planHash").to_owned();
    env.create_out_of_band_table(ns, "drifty").await;
    let response = router
        .clone()
        .oneshot(request(
            "POST",
            "/schema/v1/apply",
            Some(&env.service_key),
            format!(r#"{{"planHash":"{hash2}"}}"#).as_bytes(),
        ))
        .await
        .expect("infallible");
    assert_eq!(
        response.status(),
        StatusCode::CONFLICT,
        "out-of-band schema drift must refuse with 409"
    );

    // Unknown hash → 404
    let response = router
        .clone()
        .oneshot(request(
            "POST",
            "/schema/v1/apply",
            Some(&env.service_key),
            br#"{"planHash":"sha256:doesnotexist"}"#,
        ))
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // status reflects the apply
    let response = router
        .oneshot(request("GET", "/schema/v1/status", Some(&env.service_key), b""))
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK);
    let status = body_json(response).await;
    assert_eq!(status["enabled"], true);
    assert_eq!(status["lastApply"]["status"], "applied");

    env.reset_namespace(ns).await;
}

/// p17-c005 gate: round-trip for EVERY `ColumnType` x nullable x
/// with/without default — 36 columns provisioned through the real API, then
/// synthesized back through `GET …/ddl`, asserting each column line renders
/// quoted, with the canonical live type, correct nullability, and a DEFAULT
/// clause exactly when one was declared.
/// (spec type, canonical live spelling, a valid allowlisted default) for the
/// round-trip matrix.
const RT_TYPES: [(&str, &str, &str); 9] = [
    ("text", "text", "'x'"),
    ("integer", "integer", "0"),
    ("bigint", "bigint", "0"),
    ("numeric", "numeric", "-3.5"),
    ("boolean", "boolean", "true"),
    ("date", "date", "'2024-01-01'"),
    ("timestamptz", "timestamp with time zone", "now()"),
    ("uuid", "uuid", "gen_random_uuid()"),
    ("jsonb", "jsonb", "'{}'"),
];

/// Spec with all 36 type x nullability x default combinations.
fn rt_spec(ns: &str) -> String {
    let mut columns = vec![
        r#"{"name":"id","type":"text","nullable":false,"primaryKey":true}"#.to_owned(),
    ];
    for (ty, _, default) in &RT_TYPES {
        let d = default.replace('"', "");
        columns.push(format!(r#"{{"name":"c_{ty}_n","type":"{ty}","nullable":true}}"#));
        columns.push(format!(r#"{{"name":"c_{ty}_nn","type":"{ty}","nullable":false}}"#));
        columns.push(format!(r#"{{"name":"c_{ty}_nd","type":"{ty}","nullable":true,"default":"{d}"}}"#));
        columns.push(format!(r#"{{"name":"c_{ty}_nnd","type":"{ty}","nullable":false,"default":"{d}"}}"#));
    }
    format!(
        r#"{{"namespace":"{ns}","tables":[{{"name":"rt","tenantScoped":true,"columns":[{}]}}]}}"#,
        columns.join(",")
    )
}

#[tokio::test]
async fn ddl_round_trip_covers_every_column_type() {
    let Some(env) = TestEnv::with_keys().await else { return };
    let ns = "p17c005_rt";
    env.reset_namespace(ns).await;
    let router = env.router_enabled(&[ns]);
    let spec = rt_spec(ns);

    let response = router
        .clone()
        .oneshot(request("POST", "/schema/v1/plan", Some(&env.service_key), spec.as_bytes()))
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK, "plan must succeed");
    let hash = body_json(response).await["planHash"].as_str().expect("hash").to_owned();
    let response = router
        .clone()
        .oneshot(request(
            "POST",
            "/schema/v1/apply",
            Some(&env.service_key),
            format!(r#"{{"planHash":"{hash}"}}"#).as_bytes(),
        ))
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK, "apply must succeed");

    let response = router
        .clone()
        .oneshot(request(
            "GET",
            &format!("/schema/v1/tables/{ns}/rt/ddl"),
            Some(&env.service_key),
            b"",
        ))
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["rlsEnabled"], true);
    assert_eq!(body["rlsForced"], true);
    let ddl = body["ddl"].as_str().expect("ddl").to_owned();

    for (ty, live, _) in &RT_TYPES {
        for (suffix, not_null, has_default) in [
            ("n", false, false),
            ("nn", true, false),
            ("nd", false, true),
            ("nnd", true, true),
        ] {
            let line = ddl
                .lines()
                .find(|l| l.trim_start().starts_with(&format!("\"c_{ty}_{suffix}\"")))
                .unwrap_or_else(|| panic!("column c_{ty}_{suffix} missing from DDL:\n{ddl}"));
            assert!(
                line.contains(live),
                "c_{ty}_{suffix}: expected live type `{live}` in `{line}`"
            );
            assert_eq!(
                line.contains("NOT NULL"),
                not_null,
                "c_{ty}_{suffix}: NOT NULL mismatch in `{line}`"
            );
            assert_eq!(
                line.contains("DEFAULT"),
                has_default,
                "c_{ty}_{suffix}: DEFAULT presence mismatch in `{line}`"
            );
        }
    }
    // Stable function defaults round-trip verbatim; PK + tenant column render quoted.
    assert!(ddl.contains("DEFAULT now()"));
    assert!(ddl.contains("DEFAULT gen_random_uuid()"));
    assert!(ddl.contains("\"tenant_id\" text NOT NULL"));
    assert!(ddl.contains("PRIMARY KEY (\"id\")"));

    // Injection-shaped path segment fails BEFORE any query.
    let response = router
        .oneshot(request(
            "GET",
            "/schema/v1/tables/p17c005_rt/evil%3B%20DROP%20TABLE%20x/ddl",
            Some(&env.service_key),
            b"",
        ))
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    env.reset_namespace(ns).await;
}
