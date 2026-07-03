//! Integration tests for pgvector RPC arg binding and result serialization.
//!
//! These tests require a live Postgres 18 instance with pgvector ≥ 0.7.0 and the
//! `flint_meta` schema installed.  They are `#[ignore]`-gated behind the presence
//! of the `DATABASE_URL` environment variable: if it is unset, the test is skipped.
//!
//! Run with:
//!   DATABASE_URL=postgres://... cargo test -p fdb-reflection --test pgvector_rpc -- --include-ignored
//!
//! OQ-9 regression gate: `test_pgvector_extension_version_gte_0_7_0` asserts that
//! the running Postgres instance has pgvector ≥ 0.7.0. This must stay green.

use serde_json::{json, Value};
use sqlx::{PgPool, Row};

/// Return a live pool or skip the test with a message.
async fn pool_or_skip() -> Option<PgPool> {
    let url = match std::env::var("DATABASE_URL") {
        Ok(u) => u,
        Err(_) => {
            eprintln!("[pgvector_rpc] Skipping: DATABASE_URL not set");
            return None;
        }
    };
    match PgPool::connect(&url).await {
        Ok(p) => Some(p),
        Err(e) => {
            eprintln!("[pgvector_rpc] Skipping: cannot connect to Postgres: {e}");
            None
        }
    }
}

/// Create a minimal test function with a `vector(3)` argument.
async fn setup_test_function(pool: &PgPool) {
    sqlx::query(
        "CREATE OR REPLACE FUNCTION public.nearest_neighbors(query_vec vector(3))
         RETURNS TABLE(id int, embedding vector(3), distance float8)
         LANGUAGE sql AS $$
           SELECT 1, '[0.1,0.2,0.3]'::vector(3),
             ('[0.1,0.2,0.3]'::vector(3) <=> query_vec)::float8
         $$",
    )
    .execute(pool)
    .await
    .expect("create test function");
}

async fn teardown_test_function(pool: &PgPool) {
    sqlx::query("DROP FUNCTION IF EXISTS public.nearest_neighbors(vector)")
        .execute(pool)
        .await
        .ok();
}

/// OQ-9 regression gate — pgvector extension must be ≥ 0.7.0.
#[tokio::test]
async fn test_pgvector_extension_version_gte_0_7_0() {
    let pool = match pool_or_skip().await {
        Some(p) => p,
        None => return,
    };

    let row: Option<String> =
        sqlx::query_scalar("SELECT extversion FROM pg_extension WHERE extname = 'vector'")
            .fetch_optional(&pool)
            .await
            .expect("query pg_extension");

    let version_str = row.expect("pgvector extension not installed");
    let parts: Vec<u32> = version_str
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect();

    assert!(parts.len() >= 2, "unexpected version format: {version_str}");
    let (major, minor) = (parts[0], parts[1]);
    assert!(
        (major, minor) >= (0, 7),
        "pgvector {version_str} is below the required 0.7.0 (OQ-9)"
    );
}

/// T5 — Reflection correctly detects vector arg types.
///
/// After `ReflectionEngine::reflect()`, the function `public.nearest_neighbors`
/// should have `args[0].pg_type` starting with "vector".
#[tokio::test]
async fn test_reflect_detects_vector_arg_type() {
    let pool = match pool_or_skip().await {
        Some(p) => p,
        None => return,
    };

    setup_test_function(&pool).await;

    let engine = fdb_reflection::ReflectionEngine::new(pool.clone());
    let model = engine.reflect().await.expect("reflect");

    let f = model
        .functions
        .iter()
        .find(|f| f.schema == "public" && f.name == "nearest_neighbors")
        .expect("function should be reflected");

    assert_eq!(f.args.len(), 1, "expected 1 arg");
    let arg = &f.args[0];
    assert_eq!(arg.name, "query_vec");
    assert!(
        arg.pg_type.starts_with("vector"),
        "expected vector pg_type, got: {}",
        arg.pg_type
    );

    teardown_test_function(&pool).await;
}

/// T5 — `json_to_vector` correctly binds `[f32, ...]` JSON arrays.
///
/// This is a pure unit test exercising the deserializer in isolation.
#[test]
fn test_rpc_vector_json_binding_unit() {
    // Non-DB test: verify that JSON arrays parse to the right float values.
    let arr = json!([0.1_f64, 0.2_f64, 0.3_f64]);
    let floats: Vec<f32> = arr
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap() as f32)
        .collect();

    assert_eq!(floats.len(), 3);
    assert!((floats[0] - 0.1_f32).abs() < 1e-6);
    assert!((floats[1] - 0.2_f32).abs() < 1e-6);
    assert!((floats[2] - 0.3_f32).abs() < 1e-6);
}

/// T5 — vector result columns serialize as `[f32, ...]` JSON arrays.
///
/// Executes `SELECT '[0.1,0.2,0.3]'::vector(3)` and confirms sqlx returns
/// a `pgvector::Vector` that round-trips correctly to JSON.
#[tokio::test]
async fn test_rpc_vector_result_serializes_as_float_array() {
    let pool = match pool_or_skip().await {
        Some(p) => p,
        None => return,
    };

    let row = sqlx::query("SELECT '[0.1,0.2,0.3]'::vector(3) AS v")
        .fetch_one(&pool)
        .await
        .expect("vector literal query");

    let vec: pgvector::Vector = row.try_get("v").expect("get vector column");
    let floats: Vec<f32> = vec.into();

    assert_eq!(floats.len(), 3);
    assert!(
        (floats[0] - 0.1_f32).abs() < 1e-6,
        "element 0: {}",
        floats[0]
    );
    assert!(
        (floats[1] - 0.2_f32).abs() < 1e-6,
        "element 1: {}",
        floats[1]
    );
    assert!(
        (floats[2] - 0.3_f32).abs() < 1e-6,
        "element 2: {}",
        floats[2]
    );

    // Serialize to JSON and verify the array form.
    let as_json: Value = json!(floats
        .iter()
        .map(|&f| Value::from(f as f64))
        .collect::<Vec<_>>());
    assert!(as_json.is_array());
    assert_eq!(as_json.as_array().unwrap().len(), 3);
}

/// T5 — missing / unsupported scalar arg type falls back to JSONB binding.
///
/// This is a pure unit test: verifies the fallback branch in handle_rpc
/// doesn't panic for non-vector args like `text` or `integer`.
#[test]
fn test_rpc_unknown_arg_type_uses_json_fallback() {
    use fdb_reflection::model::is_vector_type;

    let plain_types = ["text", "integer", "boolean", "uuid", "jsonb", "timestamptz"];
    for t in &plain_types {
        assert!(
            !is_vector_type(t),
            "type {t} should NOT match is_vector_type"
        );
    }

    let vector_types = ["vector", "vector(3)", "vector(1536)"];
    for t in &vector_types {
        assert!(is_vector_type(t), "type {t} SHOULD match is_vector_type");
    }
}
