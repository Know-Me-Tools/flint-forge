//! Live-Postgres integration test for `PgRest::execute` (p35-c004).
//!
//! DATABASE_URL-gated: runs when `DATABASE_URL` is set, skips cleanly otherwise, so
//! the default `cargo test` (and CI unit stage) never require a database. The CI
//! db-integration stage (scripts/ci-test.sh with DATABASE_URL) runs it.
//!
//! Proves the fdb-query → SQL → rows path end-to-end: a filtered `RestQuery` returns
//! the expected rows, executed under the 6-GUC RLS context via `backend.acquire`.
#![allow(clippy::expect_used)]

use deadpool_postgres::{Config, Runtime};
use fdb_domain::RestQuery;
use fdb_ports::RestExecutor;
use fdb_postgres::PgRest;
use forge_identity::RlsContext;
use tokio_postgres::NoTls;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty())
}

/// An RlsContext whose role is a role the test DB already has. We use the
/// connection's own login role (via `current_user`) so `SET LOCAL ROLE` succeeds
/// without provisioning `authenticated`; the point of this test is the query
/// builder + execution path, not RLS policy enforcement (covered elsewhere).
fn rls_for_role(role: &str) -> RlsContext {
    RlsContext {
        role: role.to_owned(),
        claims_json: "{}".to_owned(),
        raw_bearer: "t".to_owned(),
        keto_subject: "test".to_owned(),
        vault_key_id: None,
    }
}

#[tokio::test]
async fn pgrest_execute_returns_filtered_rows() {
    let Some(url) = database_url() else {
        eprintln!("[pgrest_live_pg] DATABASE_URL unset — skipping");
        return;
    };

    let mut cfg = Config::new();
    cfg.url = Some(url);
    let pool = cfg
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("create pool");

    // Ephemeral schema/table; discover the current role for a valid SET LOCAL ROLE.
    let setup = pool.get().await.expect("conn");
    let role: String = setup
        .query_one("SELECT current_user", &[])
        .await
        .expect("current_user")
        .get(0);
    setup
        .batch_execute(
            "DROP SCHEMA IF EXISTS pgrest_it CASCADE; \
             CREATE SCHEMA pgrest_it; \
             CREATE TABLE pgrest_it.widget (id int PRIMARY KEY, status text); \
             INSERT INTO pgrest_it.widget VALUES (1,'active'),(2,'archived'),(3,'active');",
        )
        .await
        .inspect_err(|e| eprintln!("pgrest setup failed: {e}"))
        .expect("ephemeral setup");
    drop(setup);

    let rest = PgRest::new(pool.clone());
    let who = rls_for_role(&role);

    // SELECT * FROM pgrest_it.widget WHERE status = $1  (status = 'active')
    let q = RestQuery {
        schema: "pgrest_it".into(),
        table: "widget".into(),
        select: None,
        filters: vec![("status".into(), "eq".into(), "active".into())],
        order: None,
        limit: None,
        offset: None,
    };

    let result = rest.execute(q, &who).await.expect("execute");
    let rows = result.rows.as_array().expect("rows array");
    assert_eq!(rows.len(), 2, "two rows have status=active");
    for row in rows {
        assert_eq!(row.get("status").and_then(|v| v.as_str()), Some("active"));
    }

    // cleanup
    let c = pool.get().await.expect("conn");
    c.batch_execute("DROP SCHEMA pgrest_it CASCADE;")
        .await
        .expect("cleanup");
}
