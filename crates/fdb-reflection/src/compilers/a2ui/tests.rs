use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use super::{A2uiAssembler, AssemblerError, AssemblyContext};

async fn connect() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPool::connect(&url).await.ok()
}

/// Seed the minimal catalog + binding needed by the gate tests.
async fn seed_grid_binding(pool: &PgPool) -> sqlx::Result<()> {
    let component_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO flint_a2ui.components
         (slug, category, primitive_type, schema, is_base)
         VALUES ('data-grid', 'data-display', 'DataGrid', '{}', true)
         ON CONFLICT (slug) DO UPDATE SET primitive_type = EXCLUDED.primitive_type
         RETURNING id",
    )
    .fetch_one(pool)
    .await?;

    sqlx::query(
        "INSERT INTO flint_a2ui.bindings
         (table_schema, table_name, component_id, binding_type, config)
         VALUES ('public', 'orders', $1, 'grid', '{\"columns\": [{\"field\": \"id\", \"header\": \"ID\"}]}')
         ON CONFLICT (table_schema, table_name, binding_type) DO UPDATE
             SET config = EXCLUDED.config",
    )
    .bind(component_id)
    .execute(pool)
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_assemble_falls_back_to_grid_binding() {
    let Some(pool) = connect().await else { return };
    seed_grid_binding(&pool).await.expect("seed binding");

    let assembler = A2uiAssembler::new(pool);
    let ctx = AssemblyContext {
        event_type: "tool_call_completed".to_string(),
        event_payload: json!({
            "data_source": {"schema": "public", "table": "orders"},
            "data": [{"id": 1}]
        }),
        application_id: None,
        jwt_claims: json!({"flint": {"user_id": "anonymous-user"}}),
        surface_id: None,
    };

    let surface = assembler.assemble(&ctx).await.expect("assemble");
    let update_components = surface
        .messages
        .iter()
        .find(|m| m.op == "updateComponents")
        .expect("updateComponents message");
    let components = update_components
        .payload
        .get("components")
        .and_then(Value::as_array)
        .expect("components array");
    assert_eq!(components[0]["component"], "DataGrid");
    assert_eq!(components[0]["props"]["data_source"], "public.orders");
}

#[tokio::test]
async fn test_assemble_latency_sla() {
    let Some(pool) = connect().await else { return };
    seed_grid_binding(&pool).await.expect("seed binding");

    let assembler = A2uiAssembler::new(pool);
    let ctx = AssemblyContext {
        event_type: "tool_call_completed".to_string(),
        event_payload: json!({
            "data_source": {"schema": "public", "table": "orders"},
            "data": [{"id": 1}]
        }),
        application_id: None,
        jwt_claims: json!({"flint": {"user_id": "anonymous-user"}}),
        surface_id: None,
    };

    let start = std::time::Instant::now();
    let _surface = assembler.assemble(&ctx).await.expect("assemble");
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 500,
        "assembly took {elapsed:?}, exceeds 500ms SLA"
    );
}

#[tokio::test]
async fn test_no_binding_returns_typed_error() {
    let Some(pool) = connect().await else { return };

    let assembler = A2uiAssembler::new(pool);
    let ctx = AssemblyContext {
        event_type: "tool_call_completed".to_string(),
        event_payload: json!({
            "data_source": {"schema": "public", "table": "does_not_exist"}
        }),
        application_id: None,
        jwt_claims: json!({"flint": {"user_id": "anonymous-user"}}),
        surface_id: None,
    };

    let err = assembler.assemble(&ctx).await.unwrap_err();
    assert!(matches!(err, AssemblerError::NoBinding(_, _)));
}
