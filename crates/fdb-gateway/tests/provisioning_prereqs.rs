//! p17-c001 gate tests: Phase-0 prerequisites for the schema-provisioning API
//! (FFS-001 §8 Phase 0).
//!
//! Two independent, environment-gated checks — each skips cleanly when its
//! prerequisites are absent, so `cargo test --workspace` never requires a
//! database or key material:
//!
//! 1. **Auth proof** (needs `FLINT_GATE_JWKS_URL`, `FLINT_GATE_ISSUER`,
//!    `FLINT_GATE_AUDIENCE`, `FLINT_SERVICE_ROLE_KEY`; `FLINT_ANON_KEY`
//!    optional): the real Sansaba-minted RS256 key authenticates through
//!    `fdb_auth::rls_from_bearer` — the exact function the gateway's RLS
//!    layer and `require_provisioner` use — and lands `role =
//!    "service_role"` in `RlsContext`. The anon key lands `role = "anon"`.
//!    Successful verification also proves the issuer/audience env matches the
//!    minted claims (jsonwebtoken validates both), but the `flint-forge`
//!    values are asserted explicitly so a drifted deployment fails loudly
//!    here rather than subtly at provisioning time.
//!    Serve the JWKS locally with e.g. `npx serve sansaba-workspace/infra/keys`
//!    and point `FLINT_GATE_JWKS_URL` at the served `jwks.json`.
//!
//! 2. **Migration idempotency** (needs `DATABASE_URL`): `0015` applies
//!    cleanly twice and leaves the ledger table + `flint_provisioner` role in
//!    place. Raw `batch_execute` of the file is deliberate — every statement
//!    is `IF NOT EXISTS`-guarded, so this coexists with the sqlx migrator
//!    that applies the same file at gateway startup.
//!
//! NEVER log the tokens themselves (CLAUDE.md: never log JWT payloads).

#![allow(clippy::expect_used)]

use tokio_postgres::NoTls;

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

#[tokio::test]
async fn service_role_key_lands_service_role_and_anon_lands_anon() {
    let (Some(_jwks_url), Some(service_key)) = (
        env_nonempty("FLINT_GATE_JWKS_URL"),
        env_nonempty("FLINT_SERVICE_ROLE_KEY"),
    ) else {
        eprintln!(
            "skipping auth proof: FLINT_GATE_JWKS_URL / FLINT_SERVICE_ROLE_KEY not set"
        );
        return;
    };

    // Absent issuer/audience are missing PREREQUISITES (verify_and_build
    // cannot run without them) → skip, like every other gated test. Present
    // but wrong values are a real misconfiguration against the Sansaba key
    // set (which mints iss/aud = "flint-forge", FFS-001 §4.1) → fail loudly
    // with the mismatch named, instead of a generic Verification error below.
    let (Some(issuer), Some(audience)) = (
        env_nonempty("FLINT_GATE_ISSUER"),
        env_nonempty("FLINT_GATE_AUDIENCE"),
    ) else {
        eprintln!("skipping auth proof: FLINT_GATE_ISSUER / FLINT_GATE_AUDIENCE not set");
        return;
    };
    assert_eq!(issuer, "flint-forge", "FLINT_GATE_ISSUER must match the minted keys");
    assert_eq!(audience, "flint-forge", "FLINT_GATE_AUDIENCE must match the minted keys");

    let ctx = fdb_auth::rls_from_bearer(&service_key)
        .await
        .expect("service_role key must verify against the served JWKS");
    assert_eq!(ctx.role, "service_role");

    let claims: serde_json::Value =
        serde_json::from_str(&ctx.claims_json).expect("claims_json parses");
    assert!(
        claims.get("sub").and_then(serde_json::Value::as_str).is_some(),
        "service_role claims must carry a sub for ledger attribution"
    );

    if let Some(anon_key) = env_nonempty("FLINT_ANON_KEY") {
        let anon_ctx = fdb_auth::rls_from_bearer(&anon_key)
            .await
            .expect("anon key must verify against the served JWKS");
        assert_eq!(anon_ctx.role, "anon");
    } else {
        eprintln!("FLINT_ANON_KEY not set; anon half of the gate not exercised");
    }
}

#[tokio::test]
async fn migration_0015_is_idempotent() {
    let Some(url) = env_nonempty("DATABASE_URL") else {
        eprintln!("skipping migration idempotency: DATABASE_URL not set");
        return;
    };

    let (client, conn) = tokio_postgres::connect(&url, NoTls)
        .await
        .expect("connect to DATABASE_URL");
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("connection task ended: {e}");
        }
    });

    let sql = include_str!("../../../migrations/0015_flint_schema_provisioning.sql");
    client.batch_execute(sql).await.expect("first apply of 0015");
    client
        .batch_execute(sql)
        .await
        .expect("second apply of 0015 must be a clean no-op (idempotency gate)");

    let role_exists: bool = client
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'flint_provisioner')",
            &[],
        )
        .await
        .expect("pg_roles query")
        .get(0);
    assert!(role_exists, "flint_provisioner role must exist after 0015");

    let ledger_exists: bool = client
        .query_one(
            "SELECT to_regclass('flint_schema.provision_ledger') IS NOT NULL",
            &[],
        )
        .await
        .expect("to_regclass query")
        .get(0);
    assert!(ledger_exists, "provision_ledger must exist after 0015");

    let policy_exists: bool = client
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM pg_policies WHERE schemaname = 'flint_schema' \
             AND tablename = 'provision_ledger' AND policyname = 'provision_ledger_provisioner_all')",
            &[],
        )
        .await
        .expect("pg_policies query")
        .get(0);
    assert!(
        policy_exists,
        "provisioner ledger policy must exist (FORCE RLS with no policy would dead-letter every ledger write)"
    );
}
