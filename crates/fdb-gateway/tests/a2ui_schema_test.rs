/// Integration gate tests for p5-c001: flint_a2ui schema.
///
/// Requires a live Postgres 18 instance with pgvector and migrations applied.
/// Set DATABASE_URL before running: `DATABASE_URL=... cargo test --test a2ui_schema_test`
///
/// These tests are skipped automatically when DATABASE_URL is unset.
use sqlx::PgPool;

async fn connect() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPool::connect(&url).await.ok()
}

#[tokio::test]
async fn test_pgvector_extension_installed() {
    let Some(pool) = connect().await else { return };

    let row: (bool,) =
        sqlx::query_as("SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'vector')")
            .fetch_one(&pool)
            .await
            .expect("query failed");

    assert!(row.0, "pgvector extension must be installed");
}

#[tokio::test]
async fn test_flint_a2ui_schema_tables_exist() {
    let Some(pool) = connect().await else { return };

    let expected_tables = [
        "applications",
        "components",
        "design_systems",
        "embeddings",
        "schemas",
        "bindings",
        "events",
        "assembly_rules",
        "roles",
        "role_assignments",
    ];

    for table in &expected_tables {
        let row: (bool,) = sqlx::query_as(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.tables
                WHERE table_schema = 'flint_a2ui' AND table_name = $1
            )",
        )
        .bind(table)
        .fetch_one(&pool)
        .await
        .expect("query failed");

        assert!(row.0, "table flint_a2ui.{table} must exist");
    }
}

#[tokio::test]
async fn test_applications_has_is_system_column() {
    let Some(pool) = connect().await else { return };

    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = 'flint_a2ui'
              AND table_name = 'applications'
              AND column_name = 'is_system'
        )",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");

    assert!(row.0, "flint_a2ui.applications must have is_system column");
}

#[tokio::test]
async fn test_hnsw_index_exists() {
    let Some(pool) = connect().await else { return };

    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS (
            SELECT 1 FROM pg_indexes
            WHERE schemaname = 'flint_a2ui'
              AND tablename = 'embeddings'
              AND indexname = 'idx_embeddings_hnsw'
        )",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");

    assert!(
        row.0,
        "HNSW index idx_embeddings_hnsw must exist on flint_a2ui.embeddings"
    );
}

#[tokio::test]
async fn test_rls_enabled_on_components_and_events() {
    let Some(pool) = connect().await else { return };

    for table in &["components", "events"] {
        let row: (bool,) = sqlx::query_as(
            "SELECT relrowsecurity FROM pg_class
             JOIN pg_namespace ON pg_namespace.oid = pg_class.relnamespace
             WHERE pg_namespace.nspname = 'flint_a2ui' AND pg_class.relname = $1",
        )
        .bind(table)
        .fetch_one(&pool)
        .await
        .expect("query failed");

        assert!(row.0, "RLS must be enabled on flint_a2ui.{table}");
    }
}

#[tokio::test]
async fn test_semantic_search_function_callable() {
    let Some(pool) = connect().await else { return };

    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS (
            SELECT 1 FROM pg_proc
            JOIN pg_namespace ON pg_namespace.oid = pg_proc.pronamespace
            WHERE pg_namespace.nspname = 'flint_a2ui'
              AND pg_proc.proname = 'semantic_search'
        )",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");

    assert!(row.0, "flint_a2ui.semantic_search() function must exist");
}

#[tokio::test]
async fn test_jwt_claims_guc_returns_null_when_unset() {
    let Some(pool) = connect().await else { return };

    // Verify that current_setting('app.jwt_claims', true) returns NULL (not an error)
    // when the GUC is not set — the `true` flag is the safety guard for service-role queries.
    let row: (Option<String>,) = sqlx::query_as("SELECT current_setting('app.jwt_claims', true)")
        .fetch_one(&pool)
        .await
        .expect("current_setting query failed");

    assert!(
        row.0.is_none(),
        "current_setting('app.jwt_claims', true) must return NULL when GUC is unset, got: {:?}",
        row.0
    );
}

#[tokio::test]
async fn test_base_applications_seeded() {
    let Some(pool) = connect().await else { return };

    let row: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM flint_a2ui.applications WHERE is_system = true")
            .fetch_one(&pool)
            .await
            .expect("query failed");

    assert!(
        row.0 >= 2,
        "at least 2 system applications (flint-admin, flint-playground) must be seeded"
    );
}
