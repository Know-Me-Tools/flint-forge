//! Catalog availability and recovery when the independent notification channel is lost.
use std::sync::Arc;
use std::time::Duration;

use fdb_ports::{BackendError, SqlExecutor};
use fdb_query::QueryParam;
use fdb_reflection::{MutationGates, ReflectionEngine, ReflectionError, StateManager};
use forge_identity::RlsContext;
use futures::{stream, StreamExt};
use serde_json::{Map, Value};

struct UnusedExecutor;

#[async_trait::async_trait]
impl SqlExecutor for UnusedExecutor {
    async fn execute_raw(
        &self,
        _: &str,
        _: Vec<QueryParam>,
        _: &RlsContext,
    ) -> Result<Vec<Map<String, Value>>, BackendError> {
        panic!("schema compilation must not execute requests")
    }
}

async fn manager(engine: ReflectionEngine) -> Result<StateManager, ReflectionError> {
    StateManager::new_with_gates(
        engine,
        Arc::new(UnusedExecutor),
        // Invalid listener configuration forces an immediate connection error.
        "invalid listener URL".into(),
        MutationGates::default(),
        Arc::new(|_, _, _| stream::empty().boxed()),
    )
    .await
}

#[tokio::test]
async fn mandatory_metadata_and_optional_catalog_treat_pool_loss_differently() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unopened")
        .expect("lazy pool");
    pool.close().await;
    let engine = ReflectionEngine::new(pool);
    let catalog = engine.load_a2ui_catalog().await.expect("optional catalog");
    assert!(catalog.components.is_empty());
    assert_eq!(catalog.version, "0.0.0");
    assert!(matches!(
        manager(engine).await,
        Err(ReflectionError::Query(sqlx::Error::PoolClosed))
    ));
}

#[tokio::test]
async fn listener_loss_recompiles_and_notifies_readers_using_the_catalog_pool(
) -> Result<(), Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        return Ok(());
    };
    let pool = sqlx::PgPool::connect(&url).await?;
    let manager = Arc::new(manager(ReflectionEngine::new(pool)).await?);
    let old = manager.current();
    let mut versions = manager.subscribe_version();
    assert_eq!(*versions.borrow(), old.version);
    let listener = Arc::clone(&manager).start_listener();
    let changed = tokio::time::timeout(Duration::from_secs(30), versions.changed()).await;
    listener.abort();
    assert!(listener
        .await
        .expect_err("cancelled listener")
        .is_cancelled());
    changed??;
    let installed = manager.current();
    assert!(!Arc::ptr_eq(&old, &installed));
    assert_eq!(installed.version, *versions.borrow_and_update());
    for snapshot in [old, installed] {
        assert_eq!(snapshot.database_model.version, snapshot.version);
        assert_eq!(
            snapshot.openapi_doc["info"]["version"],
            snapshot.version.to_string()
        );
    }
    Ok(())
}
