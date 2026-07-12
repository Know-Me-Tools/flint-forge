//! Live-Postgres integration test for `PgGraphQl::execute` (jsonb result decoding).
//!
//! DATABASE_URL-gated: runs when `DATABASE_URL` is set, skips cleanly otherwise, so
//! the default `cargo test` (and CI unit stage) never require a database. Further
//! skips if `pg_graphql` isn't installed on the target database — the extension has
//! no reliable PG18 package yet (see images/postgres18/Dockerfile), so this guards
//! against environments where it's unavailable while still exercising the real
//! `graphql.resolve()` path wherever the extension is present.
//!
//! Proves `graphql.resolve()`'s jsonb return value is decoded without panicking —
//! regression test for the `row.get::<_, String>` vs. jsonb type mismatch.
#![allow(clippy::expect_used)]

use deadpool_postgres::{Config, Runtime};
use fdb_domain::GraphQlRequest;
use fdb_ports::GraphQlExecutor;
use fdb_postgres::PgGraphQl;
use forge_identity::RlsContext;
use tokio_postgres::NoTls;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty())
}

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
async fn graphql_resolve_decodes_jsonb_without_panicking() {
    let Some(url) = database_url() else {
        eprintln!("[graphql_live_pg] DATABASE_URL unset — skipping");
        return;
    };

    let mut cfg = Config::new();
    cfg.url = Some(url);
    let pool = cfg
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("create pool");

    let setup = pool.get().await.expect("conn");
    let role: String = setup
        .query_one("SELECT current_user", &[])
        .await
        .expect("current_user")
        .get(0);
    if setup
        .batch_execute("CREATE EXTENSION IF NOT EXISTS pg_graphql;")
        .await
        .is_err()
    {
        eprintln!("[graphql_live_pg] pg_graphql unavailable — skipping");
        return;
    }
    setup
        .batch_execute(
            "DROP SCHEMA IF EXISTS graphql_it CASCADE; \
             CREATE SCHEMA graphql_it; \
             CREATE TABLE graphql_it.widget (id int PRIMARY KEY, name text); \
             INSERT INTO graphql_it.widget VALUES (1,'a'),(2,'b');",
        )
        .await
        .inspect_err(|e| eprintln!("graphql setup failed: {e}"))
        .expect("ephemeral setup");
    drop(setup);

    let gql = PgGraphQl::new(pool.clone());
    let who = rls_for_role(&role);

    let req = GraphQlRequest {
        query: "query { __typename }".to_owned(),
        variables: serde_json::json!({}),
        operation_name: None,
    };

    // Must not panic (regression coverage for the jsonb/String FromSql mismatch)
    // and must return a valid JSON object.
    let result = gql.execute(req, &who).await.expect("execute");
    assert!(
        result.is_object(),
        "graphql.resolve result should be a JSON object"
    );

    let c = pool.get().await.expect("conn");
    c.batch_execute("DROP SCHEMA graphql_it CASCADE;")
        .await
        .expect("cleanup");
}
