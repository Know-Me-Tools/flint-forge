//! Live-Postgres regression test: `fdb-gateway`'s exact startup reflection path
//! (`main.rs` — `ReflectionEngine::new` -> `StateManager::new`) must succeed
//! against a real, migrated Postgres instance.
//!
//! DATABASE_URL-gated: runs when `DATABASE_URL` is set, skips otherwise, so the
//! default `cargo test` / CI unit stage never requires a database.
//!
//! # Why this test exists
//!
//! `cargo test --workspace` alone does not catch every reflection regression:
//! individual `fdb-reflection` tests call `ReflectionEngine::reflect()` (the
//! model builder), but none of them replicate `main.rs`'s actual boot sequence
//! — `StateManager::new(engine, pool, db_url).await.expect("initial schema
//! compile")` — which is the exact call that panicked on a stale `ext-flint-meta`
//! image (`flint_meta.views()` missing: PgDatabaseError 42703). A stale/partial
//! extension build can still let narrower tests pass while the real gateway
//! binary panics on line 1 of `main()`.
//!
//! This test is the direct regression gate for that incident: it builds a
//! `StateManager` exactly as `fdb-gateway::main` does and asserts the initial
//! compile succeeds, so any drift between `ext-flint-meta`'s installed SQL and
//! what `fdb-reflection` expects fails CI here — not at `docker run` in an
//! operator's environment.
#![allow(clippy::expect_used)]

use fdb_reflection::{ReflectionEngine, StateManager};
use sqlx::PgPool;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty())
}

#[tokio::test]
async fn gateway_initial_schema_compile_succeeds_against_live_pg() {
    let Some(url) = database_url() else {
        eprintln!("[gateway_startup_live_pg] DATABASE_URL unset — skipping");
        return;
    };

    // Mirrors crates/fdb-gateway/src/main.rs: connect, then build the
    // ReflectionEngine + StateManager exactly as the real binary does. No
    // gates/subscription factory are needed to prove the initial compile path
    // — MutationGates::default() and no sub_stream_factory take the same
    // engine.reflect() route through fetch_tables/fetch_functions/fetch_views.
    let pool = PgPool::connect(&url)
        .await
        .expect("gateway_startup_live_pg: connect");

    let engine = ReflectionEngine::new(pool.clone());

    let state_manager = StateManager::new(engine, pool, url).await;

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
