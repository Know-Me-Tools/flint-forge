//! p17-c006: key-rotation revocation, simulated end-to-end (FFS-001 §9).
//!
//! Rotation's observable effect is that the old key's `kid` disappears from
//! the served JWKS — so this test simulates exactly that WITHOUT touching
//! the operator's real key material: it first proves the current
//! `service_role` key verifies against the real served JWKS, then points
//! `FLINT_GATE_JWKS_URL` at a scratch in-process server whose JWKS is empty
//! (`{"keys":[]}`, the post-rotation state for the old kid) and asserts the
//! very same key is now refused.
//!
//! Lives in its OWN test binary deliberately: `FLINT_GATE_JWKS_URL` is
//! process-global and read per verification, and cargo runs test binaries in
//! separate processes — so the env mutation here cannot race the other
//! gated suites (edition 2021: `set_var` is a safe fn).
//!
//! The companion armed test in `phase_boundary_e2e.rs`
//! (`rotated_out_service_role_key_is_refused`) covers the REAL rotation
//! procedure with the operator's actual pre-rotation key.

#![allow(clippy::expect_used)]

use axum::routing::get;
use axum::{Json, Router};

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

#[tokio::test]
async fn key_dies_when_its_kid_leaves_the_jwks() {
    let (Some(service_key), Some(real_jwks)) = (
        env_nonempty("FLINT_SERVICE_ROLE_KEY"),
        env_nonempty("FLINT_GATE_JWKS_URL"),
    ) else {
        eprintln!("skipping: FLINT_SERVICE_ROLE_KEY / FLINT_GATE_JWKS_URL not set");
        return;
    };
    if env_nonempty("FLINT_GATE_ISSUER").is_none() || env_nonempty("FLINT_GATE_AUDIENCE").is_none()
    {
        eprintln!("skipping: FLINT_GATE_ISSUER / FLINT_GATE_AUDIENCE not set");
        return;
    }

    // forge-identity's JWKS cache is process-global and NOT keyed by URL
    // (jwks.rs `static JWKS`), with a 10-minute default TTL — which means a
    // rotated-out key genuinely keeps verifying on a warm gateway for up to
    // `FLINT_GATE_JWKS_TTL_SECS` after the served JWKS changes. That is the
    // real revocation latency window operators must know about (runbook
    // §14). Zero the TTL here so this test observes the post-expiry state.
    std::env::set_var("FLINT_GATE_JWKS_TTL_SECS", "0");

    // Pre-rotation: the key verifies against the real JWKS.
    fdb_auth::rls_from_bearer(&service_key)
        .await
        .expect("current key must verify against the real JWKS before the simulated rotation");

    // Scratch post-rotation JWKS: the old kid is simply gone.
    let app = Router::new().route(
        "/jwks.json",
        get(|| async { Json(serde_json::json!({ "keys": [] })) }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind scratch JWKS listener");
    let addr = listener.local_addr().expect("scratch addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    std::env::set_var("FLINT_GATE_JWKS_URL", format!("http://{addr}/jwks.json"));
    let refused = fdb_auth::rls_from_bearer(&service_key).await;
    // Restore before asserting so a failure cannot poison a rerun.
    std::env::set_var("FLINT_GATE_JWKS_URL", &real_jwks);

    assert!(
        refused.is_err(),
        "the same key must be refused once its kid is absent from the served JWKS \
         (rotation is the revocation path for a 10-year key)"
    );
}
