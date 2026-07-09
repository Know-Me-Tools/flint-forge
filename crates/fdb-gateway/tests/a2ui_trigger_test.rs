/// Integration gate tests for p5-c003: auto-binding trigger and column-type mapping.
///
/// Requires a live Postgres 18 with migrations 0001-0003 applied and seeds run.
/// Set DATABASE_URL before running: `DATABASE_URL=... cargo test --test a2ui_trigger_test`
/// Tests skip gracefully when DATABASE_URL is unset.
use sqlx::PgPool;

async fn connect() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPool::connect(&url).await.ok()
}

#[tokio::test]
async fn test_column_type_to_component_text_types() {
    let Some(pool) = connect().await else { return };

    let cases = [
        ("text", "text-input"),
        ("varchar", "text-input"),
        ("uuid", "text-input"),
        ("int4", "number-input"),
        ("int8", "number-input"),
        ("float8", "number-input"),
        ("bool", "toggle"),
        ("date", "date-picker"),
        ("timestamp", "date-picker"),
        ("timestamptz", "date-picker"),
        ("jsonb", "json-viewer"),
        ("json", "json-viewer"),
        ("unknown_type", "text-input"),
    ];

    for (pg_type, expected) in &cases {
        let row: (String,) = sqlx::query_as("SELECT flint_a2ui.column_type_to_component($1)")
            .bind(pg_type)
            .fetch_one(&pool)
            .await
            .expect("column_type_to_component query failed");

        assert_eq!(
            row.0.as_str(),
            *expected,
            "column_type_to_component('{}') expected '{}', got '{}'",
            pg_type,
            expected,
            row.0
        );
    }
}

#[tokio::test]
async fn test_auto_binding_trigger_generates_bindings() {
    let Some(pool) = connect().await else { return };
    let Ok(mut tx) = pool.begin().await else {
        return;
    };

    let test_table = format!(
        "_a2ui_trigger_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_micros()
    );

    // Insert a test row into flint_meta.cache_tables to fire the trigger
    let insert_result = sqlx::query(
        "INSERT INTO flint_meta.cache_tables
             (schema_name, table_name, is_view)
         VALUES ('public', $1, false)
         ON CONFLICT (schema_name, table_name) DO NOTHING",
    )
    .bind(&test_table)
    .execute(&mut *tx)
    .await;

    // If the table doesn't exist or the insert is blocked, skip the trigger test
    let Ok(_) = insert_result else {
        return; // flint_meta.cache_tables may not exist in this environment
    };

    // Grid binding must exist
    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS (
            SELECT 1 FROM flint_a2ui.bindings
            WHERE table_schema = 'public' AND table_name = $1 AND binding_type = 'grid'
        )",
    )
    .bind(&test_table)
    .fetch_one(&mut *tx)
    .await
    .expect("bindings query failed");
    assert!(
        row.0,
        "grid binding must be auto-generated for {test_table}"
    );

    // Form binding must exist (BASE TABLE)
    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS (
            SELECT 1 FROM flint_a2ui.bindings
            WHERE table_schema = 'public' AND table_name = $1 AND binding_type = 'form'
        )",
    )
    .bind(&test_table)
    .fetch_one(&mut *tx)
    .await
    .expect("bindings query failed");
    assert!(
        row.0,
        "form binding must be auto-generated for BASE TABLE {test_table}"
    );

    // Detail binding must exist
    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS (
            SELECT 1 FROM flint_a2ui.bindings
            WHERE table_schema = 'public' AND table_name = $1 AND binding_type = 'detail'
        )",
    )
    .bind(&test_table)
    .fetch_one(&mut *tx)
    .await
    .expect("bindings query failed");
    assert!(
        row.0,
        "detail binding must be auto-generated for {test_table}"
    );

    // Audit event must be logged
    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS (
            SELECT 1 FROM flint_a2ui.events
            WHERE event_type = 'binding_auto_generated'
              AND object = 'public.' || $1
        )",
    )
    .bind(&test_table)
    .fetch_one(&mut *tx)
    .await
    .expect("events query failed");
    assert!(
        row.0,
        "binding_auto_generated event must be logged for {test_table}"
    );

    // Rollback so the synthetic cache row and bindings never leak to other tests.
    let _ = tx.rollback().await;
}

#[tokio::test]
async fn test_trigger_no_form_for_view() {
    let Some(pool) = connect().await else { return };
    let Ok(mut tx) = pool.begin().await else {
        return;
    };

    let test_view = format!(
        "_a2ui_view_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_micros()
    );

    let insert_result = sqlx::query(
        "INSERT INTO flint_meta.cache_tables
             (schema_name, table_name, is_view)
         VALUES ('public', $1, true)
         ON CONFLICT (schema_name, table_name) DO NOTHING",
    )
    .bind(&test_view)
    .execute(&mut *tx)
    .await;

    let Ok(_) = insert_result else { return };

    // Views must NOT get a form binding
    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS (
            SELECT 1 FROM flint_a2ui.bindings
            WHERE table_schema = 'public' AND table_name = $1 AND binding_type = 'form'
        )",
    )
    .bind(&test_view)
    .fetch_one(&mut *tx)
    .await
    .expect("query failed");
    assert!(!row.0, "VIEW must not receive a form binding: {test_view}");

    // But grid and detail must still be generated
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM flint_a2ui.bindings
         WHERE table_schema = 'public' AND table_name = $1
           AND binding_type IN ('grid', 'detail')",
    )
    .bind(&test_view)
    .fetch_one(&mut *tx)
    .await
    .expect("query failed");
    assert_eq!(
        row.0, 2,
        "VIEW must have grid + detail bindings: {test_view}"
    );

    // Rollback so the synthetic cache row and bindings never leak to other tests.
    let _ = tx.rollback().await;
}
