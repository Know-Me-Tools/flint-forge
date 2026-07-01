/// Integration gate tests for p5-c002: base component seed.
///
/// Requires a live Postgres 18 with migrations + seed applied.
/// Set DATABASE_URL before running: `DATABASE_URL=... cargo test --test a2ui_seed_test`
/// Tests skip gracefully when DATABASE_URL is unset.
use sqlx::PgPool;

async fn connect() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPool::connect(&url).await.ok()
}

#[tokio::test]
async fn test_base_component_count_at_least_50() {
    let Some(pool) = connect().await else { return };

    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM flint_a2ui.components WHERE is_base = true",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");

    assert!(
        row.0 >= 50,
        "expected at least 50 base components, got {}",
        row.0
    );
}

#[tokio::test]
async fn test_all_seven_categories_present() {
    let Some(pool) = connect().await else { return };

    let categories = ["layout", "data-display", "input", "action", "navigation", "feedback", "system"];

    for cat in &categories {
        let row: (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM flint_a2ui.components WHERE category = $1 AND is_base = true)",
        )
        .bind(cat)
        .fetch_one(&pool)
        .await
        .expect("query failed");

        assert!(row.0, "category '{}' must have at least one base component", cat);
    }
}

#[tokio::test]
async fn test_key_components_present() {
    let Some(pool) = connect().await else { return };

    let required = ["data-grid", "form", "text-input", "button", "modal", "nav-bar"];

    for slug in &required {
        let row: (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM flint_a2ui.components WHERE slug = $1)",
        )
        .bind(slug)
        .fetch_one(&pool)
        .await
        .expect("query failed");

        assert!(row.0, "required component '{}' must exist", slug);
    }
}

#[tokio::test]
async fn test_flint_meta_schema_system_component_exists() {
    let Some(pool) = connect().await else { return };

    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM flint_a2ui.components WHERE slug = 'flint-meta-schema' AND category = 'system')",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");

    assert!(row.0, "'flint-meta-schema' system component must exist");
}

#[tokio::test]
async fn test_all_base_components_have_valid_json_schema() {
    let Some(pool) = connect().await else { return };

    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM flint_a2ui.components
         WHERE is_base = true
           AND (jsonb_typeof(schema) != 'object' OR NOT (schema ? 'type'))",
    )
    .fetch_one(&pool)
    .await
    .expect("query failed");

    assert_eq!(
        row.0, 0,
        "{} base components have invalid schema (must be object with 'type' key)",
        row.0
    );
}
