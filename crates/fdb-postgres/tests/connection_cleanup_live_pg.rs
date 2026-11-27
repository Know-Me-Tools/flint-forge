//! Real PostgreSQL cancellation/rollback regressions. No persistent tables.
#![allow(clippy::expect_used)]

use deadpool_postgres::{Config, Pool, Runtime};
use fdb_ports::SqlExecutor;
use fdb_postgres::conn::PgConn;
use fdb_postgres::PgRest;
use forge_identity::RlsContext;
use std::time::Duration;
use tokio_postgres::NoTls;

fn pool() -> Option<Pool> {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|url| !url.is_empty())?;
    let mut config = Config::new();
    config.url = Some(url);
    let pool = config
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("pool");
    pool.resize(1);
    Some(pool)
}

async fn assert_lock_released(observer: &deadpool_postgres::Object, key: i64) {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let free: bool = observer
                .query_one("SELECT pg_try_advisory_xact_lock($1)", &[&key])
                .await
                .expect("independent lock probe")
                .get(0);
            if free {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("dropped transaction must release its lock without another pool checkout");
}

#[tokio::test]
async fn abandoned_transaction_releases_lock_and_does_not_recycle_local_claims() {
    let (Some(pool), Some(other)) = (pool(), pool()) else {
        return;
    };
    let observer = other.get().await.expect("independent connection");
    let object = pool.get().await.expect("checkout");
    let pid: i32 = object
        .query_one("SELECT pg_backend_pid()", &[])
        .await
        .expect("pid")
        .get(0);
    let key = 7_540_100_000_i64 + i64::from(pid);
    object
        .batch_execute("BEGIN; SET LOCAL request.jwt.claims='cleanup-test-only'")
        .await
        .expect("begin");
    object
        .query_one("SELECT pg_advisory_xact_lock($1)", &[&key])
        .await
        .expect("lock");
    drop(PgConn::new(object));
    assert_lock_released(&observer, key).await;
    let fresh = pool.get().await.expect("fresh checkout");
    let row = fresh
        .query_one(
            "SELECT pg_backend_pid(), current_setting('request.jwt.claims',true)",
            &[],
        )
        .await
        .expect("clean scope");
    assert_ne!(
        row.get::<_, i32>(0),
        pid,
        "unfinished connection must be discarded"
    );
    assert!(row
        .get::<_, Option<String>>(1)
        .is_none_or(|value| value.is_empty()));
}

#[tokio::test]
async fn aborted_owner_releases_lock_without_detached_cleanup_task() {
    let (Some(pool), Some(other)) = (pool(), pool()) else {
        return;
    };
    let observer = other.get().await.expect("independent connection");
    let (sent, received) = tokio::sync::oneshot::channel();
    let task_pool = pool.clone();
    let owner = tokio::spawn(async move {
        let object = task_pool.get().await.expect("checkout");
        let pid: i32 = object
            .query_one("SELECT pg_backend_pid()", &[])
            .await
            .expect("pid")
            .get(0);
        let key = 7_540_200_000_i64 + i64::from(pid);
        object.batch_execute("BEGIN").await.expect("begin");
        object
            .query_one("SELECT pg_advisory_xact_lock($1)", &[&key])
            .await
            .expect("lock");
        let connection = PgConn::new(object);
        sent.send(key).expect("ready");
        std::future::pending::<()>().await;
        drop(connection);
    });
    let key = received.await.expect("actual transaction ready");
    owner.abort();
    assert!(owner.await.expect_err("cancelled task").is_cancelled());
    assert_lock_released(&observer, key).await;
}

#[tokio::test]
async fn committed_transaction_keeps_pool_reuse_and_clears_local_claims() {
    let Some(pool) = pool() else {
        return;
    };
    let object = pool.get().await.expect("checkout");
    let pid: i32 = object
        .query_one("SELECT pg_backend_pid()", &[])
        .await
        .expect("pid")
        .get(0);
    object.batch_execute("BEGIN; SET LOCAL request.jwt.claims='cleanup-test-only'; CREATE TEMP TABLE cleanup_commit(value integer); INSERT INTO cleanup_commit VALUES(7)").await.expect("transaction");
    let connection = PgConn::new(object);
    connection.commit().await.expect("commit");
    drop(connection);
    let reused = pool.get().await.expect("reuse");
    let row = reused.query_one("SELECT pg_backend_pid(), current_setting('request.jwt.claims',true), (SELECT value FROM cleanup_commit)", &[]).await.expect("durable commit");
    assert_eq!(row.get::<_, i32>(0), pid);
    assert!(row
        .get::<_, Option<String>>(1)
        .is_none_or(|value| value.is_empty()));
    assert_eq!(row.get::<_, i32>(2), 7);
}

#[tokio::test]
async fn explicitly_rolled_back_query_error_keeps_a_clean_reusable_connection() {
    let Some(pool) = pool() else {
        return;
    };
    let object = pool.get().await.expect("checkout");
    let row = object
        .query_one("SELECT pg_backend_pid(),current_user", &[])
        .await
        .expect("identity");
    let pid: i32 = row.get(0);
    let role: String = row.get(1);
    drop(object);
    let executor = PgRest::new(pool.clone());
    let scope = RlsContext {
        role,
        claims_json: "{}".to_owned(),
        raw_bearer: "cleanup-test-only".to_owned(),
        keto_subject: "cleanup-test-only".to_owned(),
        vault_key_id: None,
    };
    assert!(executor
        .execute_raw("SELECT 1/0", Vec::new(), &scope)
        .await
        .is_err());
    let reused = pool.get().await.expect("reuse after rollback");
    let row = reused
        .query_one(
            "SELECT pg_backend_pid(),current_setting('request.jwt.claims',true)",
            &[],
        )
        .await
        .expect("clean scope");
    assert_eq!(row.get::<_, i32>(0), pid);
    assert!(row
        .get::<_, Option<String>>(1)
        .is_none_or(|value| value.is_empty()));
}
