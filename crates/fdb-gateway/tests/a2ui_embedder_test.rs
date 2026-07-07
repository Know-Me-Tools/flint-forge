/// Integration gate tests for p5-c004: A2UI component embeddings pipeline.
///
/// Requires a live Postgres 18 with migrations 0001-0005 applied and seeds run.
/// The `ext-flint-llm` extension (or a compatible `llm.embed(text,model)`
/// function) must be installed for the full semantic-search assertions.
///
/// Set DATABASE_URL before running:
///     DATABASE_URL=... cargo test --test a2ui_embedder_test
/// Tests skip gracefully when DATABASE_URL is unset.
use std::sync::Arc;

use fdb_gateway::a2ui_embedder;
use sqlx::PgPool;

async fn connect() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPool::connect(&url).await.ok()
}

#[tokio::test]
async fn test_hybrid_search_function_exists() {
    let Some(pool) = connect().await else { return };

    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS (
            SELECT 1 FROM pg_proc p
            JOIN pg_namespace n ON n.oid = p.pronamespace
            WHERE n.nspname = 'flint_a2ui' AND p.proname = 'hybrid_search'
        )",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");

    assert!(row.0, "flint_a2ui.hybrid_search() function must exist");
}

#[tokio::test]
async fn test_backfill_and_semantic_search_for_data_grid() {
    let Some(pool) = connect().await else { return };

    if !llm_embed_available(&pool).await {
        eprintln!("skipping semantic search test: llm.embed() is unavailable");
        return;
    }

    // Run the same backfill routine the gateway spawns on startup.
    let pool = Arc::new(pool);
    a2ui_embedder::backfill_missing(&pool)
        .await
        .expect("backfill should complete");

    let rows: Vec<(String, f64)> = sqlx::query_as(
        "SELECT slug, score
         FROM flint_a2ui.hybrid_search('sortable table with pagination', 10)
         JOIN flint_a2ui.components c ON c.id = hybrid_search.component_id",
    )
    .fetch_all(&*pool)
    .await
    .expect("hybrid_search query failed");

    assert!(
        !rows.is_empty(),
        "hybrid_search should return results for 'sortable table with pagination'"
    );
    let top_slug = &rows[0].0;
    assert_eq!(
        top_slug, "data-grid",
        "expected 'data-grid' as top semantic result, got '{top_slug}'"
    );
}

#[tokio::test]
async fn test_hybrid_search_for_date_picker() {
    let Some(pool) = connect().await else { return };

    if !llm_embed_available(&pool).await {
        eprintln!("skipping semantic search test: llm.embed() is unavailable");
        return;
    }

    let pool = Arc::new(pool);
    a2ui_embedder::backfill_missing(&pool)
        .await
        .expect("backfill should complete");

    let rows: Vec<(String, f64)> = sqlx::query_as(
        "SELECT slug, score
         FROM flint_a2ui.hybrid_search('date selection field', 10)
         JOIN flint_a2ui.components c ON c.id = hybrid_search.component_id",
    )
    .fetch_all(&*pool)
    .await
    .expect("hybrid_search query failed");

    assert!(
        !rows.is_empty(),
        "hybrid_search should return results for 'date selection field'"
    );
    let top_slug = &rows[0].0;
    assert_eq!(
        top_slug, "date-picker",
        "expected 'date-picker' as top hybrid result, got '{top_slug}'"
    );
}

/// Best-effort probe for the in-database `llm.embed()` function.
async fn llm_embed_available(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, Option<String>>(
        "SELECT llm.embed('probe', 'text-embedding-3-small')::text",
    )
    .fetch_one(pool)
    .await
    .ok()
    .flatten()
    .is_some()
}
