//! Live-Postgres regression test: `fdb-gateway`'s exact startup reflection path
//! (`main.rs` — `ReflectionEngine::new` -> `StateManager::new_with_gates`) must
//! succeed against a real, migrated Postgres instance.
//!
//! DATABASE_URL-gated: runs when `DATABASE_URL` is set, skips otherwise, so the
//! default `cargo test` / CI unit stage never requires a database.
//!
//! # Why this test exists
//!
//! `cargo test --workspace` alone does not catch every reflection regression:
//! individual `fdb-reflection` tests call `ReflectionEngine::reflect()` (the
//! model builder), but none of them replicate `main.rs`'s actual boot sequence
//! — `StateManager::new_with_gates(..).await.expect("initial schema compile")`
//! — which is the exact call that panicked on a stale `ext-flint-meta` image
//! (`flint_meta.views()` missing: PgDatabaseError 42703). A stale/partial
//! extension build can still let narrower tests pass while the real gateway
//! binary panics on line 1 of `main()`.
//!
//! This test is the direct regression gate for that incident: it builds a
//! `StateManager` exactly as `fdb-gateway::main` does and asserts the initial
//! compile succeeds, so any drift between `ext-flint-meta`'s installed SQL and
//! what `fdb-reflection` expects fails CI here — not at `docker run` in an
//! operator's environment.
#![allow(clippy::expect_used)]

use fdb_postgres::PgRest;
use fdb_reflection::{MutationGates, ReflectionEngine, StateManager};
use futures::stream::{self, StreamExt};
use sqlx::PgPool;
use std::sync::Arc;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty())
}

#[tokio::test]
async fn gateway_initial_schema_compile_succeeds_against_live_pg() {
    let Some(url) = database_url() else {
        eprintln!("[gateway_startup_live_pg] DATABASE_URL unset — skipping");
        return;
    };

    // Mirrors crates/fdb-gateway/src/bootstrap.rs: connect the privileged
    // reflection pool, build the RLS-scoped SqlExecutor (PgRest over a
    // separate deadpool_postgres pool — never the raw sqlx pool, see
    // bootstrap.rs's p16-c001 comment) + StateManager exactly as the real
    // binary does. The subscription live-stream factory is mandatory (see
    // state_manager.rs) — an empty factory proves the initial compile path
    // without needing a real Quarry/ChangeStreamSource wired up.
    let pool = PgPool::connect(&url)
        .await
        .expect("gateway_startup_live_pg: connect");

    let rest_pool = {
        let mut cfg = deadpool_postgres::Config::new();
        cfg.url = Some(url.clone());
        cfg.create_pool(
            Some(deadpool_postgres::Runtime::Tokio1),
            tokio_postgres::NoTls,
        )
        .expect("gateway_startup_live_pg: rest-executor pool create")
    };
    let executor: Arc<dyn fdb_ports::SqlExecutor> = Arc::new(PgRest::new(rest_pool));

    let engine = ReflectionEngine::new(pool);
    let empty_factory = Arc::new(|_spec, _meta, _who| {
        stream::empty::<async_graphql::Result<async_graphql::Value>>().boxed()
    });

    let state_manager = StateManager::new_with_gates(
        engine,
        executor,
        url,
        MutationGates::default(),
        empty_factory,
    )
    .await;

    assert!(
        state_manager.is_ok(),
        "fdb-gateway's initial schema compile failed against a live Postgres \
         instance: {:?}\n\
         This is the exact failure mode from the ext-flint-meta stale-image \
         incident — a flint_meta.* reflection function is missing or its \
         signature has drifted from what fdb-reflection/src/engine.rs expects. \
         Rebuild images/postgres18/Dockerfile from current source and re-run.",
        state_manager.err()
    );

    // The compiled state must actually contain the reflection-critical model
    // shapes, not just return Ok with an empty/degenerate model.
    let state = state_manager.expect("checked above").current();
    assert!(
        state.database_model.version > 0,
        "compiled DatabaseModel must have a version > 0 after the initial compile"
    );
}
