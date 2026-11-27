#![allow(clippy::expect_used)]

use crate::PgBackend;
use deadpool_postgres::{Config, Runtime};
use fdb_ports::DatabaseBackend;
use forge_identity::RlsContext;
use tokio_postgres::NoTls;

#[tokio::test]
async fn failed_role_setup_discards_the_open_transaction() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        return;
    };
    let mut config = Config::new();
    config.url = Some(url);
    let pool = config
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("pool");
    pool.resize(1);
    let original = pool.get().await.expect("checkout");
    let pid: i32 = original
        .query_one("SELECT pg_backend_pid()", &[])
        .await
        .expect("pid")
        .get(0);
    drop(original);
    let backend = PgBackend { pool: pool.clone() };
    let invalid = RlsContext {
        role: "invalid-role".to_owned(),
        claims_json: "{}".to_owned(),
        raw_bearer: "cleanup-test-only".to_owned(),
        keto_subject: "cleanup-test-only".to_owned(),
        vault_key_id: None,
    };
    assert!(backend.acquire(&invalid).await.is_err());
    let fresh = pool.get().await.expect("fresh connection");
    let new_pid: i32 = fresh
        .query_one("SELECT pg_backend_pid()", &[])
        .await
        .expect("new pid")
        .get(0);
    assert_ne!(
        pid, new_pid,
        "failure after BEGIN must discard the connection"
    );
}
