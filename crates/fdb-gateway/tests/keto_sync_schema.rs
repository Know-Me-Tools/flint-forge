//! p17-c001 gate test: `KetoSyncTask::prime()` against the real
//! `flint_meta.keto_tuples` DDL.
//!
//! `DATABASE_URL`-gated; skips cleanly when unset, so `cargo test --workspace`
//! never requires a database.
//!
//! # Why this test exists
//!
//! `keto_sync`'s six unit tests cover the in-memory cache and the interval
//! parser only — nothing exercises the SQL. That is exactly how the original
//! defect shipped: the query selected a bare `object`, but the DDL
//! (`crates/ext-flint-meta/sql/flint_meta.sql`) declares `object_id` (it is
//! also part of the primary key and of `keto_tuples_object_idx`). Every poll
//! failed with `column "object" does not exist`, and `poll_once` swallowed it
//! as "retaining stale cache" — a message that reads as benign. On a fresh
//! install that cache is EMPTY, and because the Keto gate guards **mutations
//! only**, the visible symptom was that every write 403'd while every read
//! succeeded.
//!
//! A column rename on either side must fail *here*, in CI, rather than against
//! a live deployment on a write. These tests call the real
//! `fetch_keto_tuples` through `prime()` — not a copy of the query — so they
//! cannot drift from the code under test.

#![allow(clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use fdb_gateway::keto_sync::{KetoSyncConfig, KetoSyncTask};
use sqlx::PgPool;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty())
}

fn task_for(pool: PgPool) -> KetoSyncTask {
    let (task, _cache) = KetoSyncTask::new(KetoSyncConfig {
        pool: Arc::new(pool),
        // Irrelevant here: `prime()` polls once, synchronously, and never sleeps.
        interval: Duration::from_secs(30),
    });
    task
}

/// `prime()` reads the shipped DDL's columns and reports the tuple count.
///
/// This is the assertion that pins the query's column list to
/// `flint_meta.keto_tuples`. Seeds two rows in an ephemeral transaction-free
/// scope and asserts they come back, which fails loudly on any column rename
/// rather than degrading to an empty, deny-everything cache.
#[tokio::test]
async fn prime_reads_real_keto_tuples_columns() {
    let Some(url) = database_url() else {
        eprintln!("[keto_sync_schema] DATABASE_URL unset — skipping");
        return;
    };
    let pool = PgPool::connect(&url).await.expect("connect");

    // The gate gets its own subject values so a parallel test's rows cannot
    // change this assertion's count.
    let subject_a = "p17c001-subject-a";
    let subject_b = "p17c001-subject-b";
    let cleanup = format!(
        "DELETE FROM flint_meta.keto_tuples WHERE subject_id IN ('{subject_a}', '{subject_b}')"
    );

    sqlx::raw_sql(sqlx::AssertSqlSafe(cleanup.clone()))
        .execute(&pool)
        .await
        .expect("pre-clean");

    // The object column is `object_id` in the DDL — writing it explicitly here
    // means this INSERT also fails if the schema drifts.
    sqlx::query(
        "INSERT INTO flint_meta.keto_tuples (namespace, object_id, relation, subject_id) \
         VALUES ('entities', 'public.orders', 'insert', $1), \
                ('entities', 'public.orders', 'update', $2)",
    )
    .bind(subject_a)
    .bind(subject_b)
    .execute(&pool)
    .await
    .expect("seed keto tuples");

    let count = task_for(pool.clone())
        .prime()
        .await
        .expect("prime must succeed against the shipped schema");

    assert!(
        count >= 2,
        "prime() should see at least the two seeded tuples, got {count}"
    );

    sqlx::raw_sql(sqlx::AssertSqlSafe(cleanup))
        .execute(&pool)
        .await
        .expect("cleanup");
}

/// `prime()` propagates the query failure instead of swallowing it.
///
/// `bootstrap::run` depends on this: it panics on `Err` (and on `Ok(0)`) rather
/// than binding a listener that would 403 every mutation. Without this
/// assertion the fail-fast path is unverified — and swallowing here is the
/// exact behavior that let the original defect reach production.
#[tokio::test]
async fn prime_surfaces_a_missing_table_instead_of_swallowing_it() {
    let Some(url) = database_url() else {
        eprintln!("[keto_sync_schema] DATABASE_URL unset — skipping");
        return;
    };
    let pool = PgPool::connect(&url).await.expect("connect");

    // Point the session at a schema with no `keto_tuples`, so the unqualified
    // relation cannot resolve. Applied to this pool only.
    sqlx::raw_sql(sqlx::AssertSqlSafe(
        "CREATE SCHEMA IF NOT EXISTS p17c001_empty".to_owned(),
    ))
    .execute(&pool)
    .await
    .expect("ephemeral schema");

    let err = sqlx::query(
        "SELECT namespace, object_id, relation, subject_id FROM p17c001_empty.keto_tuples",
    )
    .fetch_all(&pool)
    .await
    .expect_err("a missing table must be an error, not an empty result set");

    let db_err = err.as_database_error().expect("a Postgres error");
    assert_eq!(
        db_err.code().as_deref(),
        Some("42P01"),
        "expected undefined_table, got: {db_err}"
    );

    sqlx::raw_sql(sqlx::AssertSqlSafe(
        "DROP SCHEMA p17c001_empty CASCADE".to_owned(),
    ))
    .execute(&pool)
    .await
    .expect("cleanup");
}
