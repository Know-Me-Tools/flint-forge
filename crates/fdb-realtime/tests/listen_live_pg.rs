//! Live-Postgres integration tests for the LISTEN/NOTIFY change source.
//!
//! These are `#[ignore]`d by default — they require a real Postgres and are NOT
//! part of the standard `cargo test` / CI gate (which has no DB). Run explicitly:
//!
//! ```bash
//! DATABASE_URL=postgres://user@localhost/db cargo test -p fdb-realtime --test listen_live_pg -- --ignored
//! ```
//!
//! Each test sets up its own ephemeral schema/table, applies the `flint.*` NOTIFY
//! functions from migration 0006, and cleans up on the way out. They prove the DB
//! half (trigger → payload) and the adapter half (`watch()` → `ChangeEvent`) that
//! the pure unit tests cannot cover.
#![allow(clippy::expect_used)]

use std::time::Duration;

use futures::StreamExt;
use sqlx::postgres::PgListener;
use sqlx::{Executor, PgPool};

/// The `flint.*` DDL from migration 0006 (idempotent). Inlined so the test does
/// not depend on the migrator's file discovery from a test binary.
const MIGRATION_0006: &str = include_str!("../../../migrations/0006_change_notify.sql");

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok()
}

/// Apply migration 0006's functions/procedure to `pool`.
///
/// The harness runs these tests in parallel against the same DB, and concurrent
/// idempotent DDL (`CREATE SCHEMA IF NOT EXISTS`, `CREATE OR REPLACE FUNCTION`)
/// races at the catalog level (`23505` / "tuple concurrently updated"). Serialize
/// the whole apply under a session-scoped Postgres advisory lock so exactly one
/// test session runs the DDL at a time — deterministic, no error-code guessing.
async fn ensure_notify_ddl(pool: &PgPool) {
    // Arbitrary fixed key identifying this test's DDL critical section.
    const LOCK_KEY: i64 = 0x0006_0006;
    let mut conn = pool.acquire().await.expect("acquire for DDL lock");
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(LOCK_KEY)
        .execute(&mut *conn)
        .await
        .expect("advisory lock");
    let res = conn.execute(MIGRATION_0006).await;
    // Always release the lock, even on DDL failure.
    let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(LOCK_KEY)
        .execute(&mut *conn)
        .await;
    res.expect("apply 0006 notify DDL");
}

#[tokio::test]
#[ignore = "requires a live Postgres (DATABASE_URL); run with --ignored"]
async fn trigger_notifies_insert_update_delete_payloads() {
    let Some(url) = database_url() else {
        eprintln!("DATABASE_URL unset — skipping");
        return;
    };
    let pool = PgPool::connect(&url).await.expect("connect");
    ensure_notify_ddl(&pool).await;

    // Ephemeral table in a throwaway schema so we never touch real data.
    pool.execute(
        "DROP SCHEMA IF EXISTS flint_listen_it CASCADE; \
         CREATE SCHEMA flint_listen_it; \
         CREATE TABLE flint_listen_it.widget (id int PRIMARY KEY, name text);",
    )
    .await
    .expect("ephemeral table");

    // Opt the table in.
    pool.execute("CALL flint.enable_change_notify('flint_listen_it', 'widget');")
        .await
        .expect("enable_change_notify");

    // Attach the listener BEFORE writing (NOTIFY only fires on commit; a listener
    // attached after the commit would miss it).
    let mut listener = PgListener::connect(&url).await.expect("listener connect");
    listener.listen("flint_change").await.expect("listen");

    pool.execute("INSERT INTO flint_listen_it.widget (id, name) VALUES (1, 'a');")
        .await
        .expect("insert");
    pool.execute("UPDATE flint_listen_it.widget SET name = 'b' WHERE id = 1;")
        .await
        .expect("update");
    pool.execute("DELETE FROM flint_listen_it.widget WHERE id = 1;")
        .await
        .expect("delete");

    // Collect the three notifications (with a timeout so a miss fails loudly).
    let mut ops = Vec::new();
    for _ in 0..3 {
        let notif = tokio::time::timeout(Duration::from_secs(5), listener.recv())
            .await
            .expect("notification within 5s")
            .expect("recv ok");
        let payload: serde_json::Value =
            serde_json::from_str(notif.payload()).expect("payload is JSON");
        assert_eq!(payload["schema"], "flint_listen_it");
        assert_eq!(payload["table"], "widget");
        assert_eq!(payload["truncated"], false);
        ops.push(payload["op"].as_str().expect("op str").to_owned());
    }
    assert_eq!(ops, vec!["insert", "update", "delete"]);

    pool.execute("DROP SCHEMA flint_listen_it CASCADE;")
        .await
        .expect("cleanup");
}

#[tokio::test]
#[ignore = "requires a live Postgres (DATABASE_URL); run with --ignored"]
async fn listen_change_source_watch_delivers_event() {
    use fdb_domain::SubscriptionSpec;
    use fdb_ports::ChangeStreamSource;
    use fdb_realtime::{KetoConfig, ListenChangeSource, ListenConfig};
    use forge_identity::RlsContext;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let Some(url) = database_url() else {
        eprintln!("DATABASE_URL unset — skipping");
        return;
    };
    let pool = PgPool::connect(&url).await.expect("connect");
    ensure_notify_ddl(&pool).await;
    pool.execute(
        "DROP SCHEMA IF EXISTS flint_listen_it2 CASCADE; \
         CREATE SCHEMA flint_listen_it2; \
         CREATE TABLE flint_listen_it2.doc (id int PRIMARY KEY, body text); \
         CALL flint.enable_change_notify('flint_listen_it2', 'doc');",
    )
    .await
    .expect("ephemeral setup");

    // Keto stub that ALLOWS the coarse check.
    let keto = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"allowed": true})),
        )
        .mount(&keto)
        .await;

    let source = ListenChangeSource::new(
        ListenConfig {
            database_url: url.clone(),
            broadcast_capacity: 64,
        },
        KetoConfig {
            base_url: keto.uri(),
        },
    )
    .await
    .expect("listen source");

    let who = RlsContext {
        role: "authenticated".into(),
        claims_json: "{}".into(),
        raw_bearer: "t".into(),
        keto_subject: "user-1".into(),
        vault_key_id: None,
    };
    let spec = SubscriptionSpec {
        tenant: String::new(),
        entity_type: "flint_listen_it2.doc".into(),
        filter: None,
    };
    let mut stream = source.watch(spec, &who).await.expect("watch opens");

    // Write AFTER watch() has subscribed to the fan-out.
    pool.execute("INSERT INTO flint_listen_it2.doc (id, body) VALUES (7, 'hi');")
        .await
        .expect("insert");

    let event = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("event within 5s")
        .expect("stream not ended")
        .expect("ok change event");
    assert_eq!(event.schema, "flint_listen_it2");
    assert_eq!(event.table, "doc");
    assert_eq!(
        event
            .record
            .as_ref()
            .and_then(|r| r.get("id"))
            .and_then(serde_json::Value::as_i64),
        Some(7)
    );

    pool.execute("DROP SCHEMA flint_listen_it2 CASCADE;")
        .await
        .expect("cleanup");
}
