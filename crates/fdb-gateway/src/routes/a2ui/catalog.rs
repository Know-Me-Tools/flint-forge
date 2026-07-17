//! A2UI catalog handler.
//!
//! - `GET    /a2ui/v1/catalog/{*catalog_id}`

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde_json::{json, Value};
use sqlx::types::Json as SqlxJson;

use super::helpers::internal_error;
use super::A2uiState;

/// `GET /a2ui/v1/catalog/{*catalog_id}`
///
/// Serves the A2UI catalog as a JSON Schema compatible with A2UI v0.9.1 and
/// CopilotKit's `<CopilotKit a2ui={{ catalog }}>` prop.
pub async fn get_catalog(
    State(state): State<A2uiState>,
    Path(catalog_id): Path<String>,
) -> impl IntoResponse {
    // catalog_id is expected as "slug/version" or "slug".
    let (slug, version) = catalog_id.split_once('/').map_or_else(
        || (catalog_id.clone(), "1.0.0".to_string()),
        |(s, v)| (s.to_string(), v.to_string()),
    );

    let components: Vec<(String, String, SqlxJson<Value>, Option<String>)> = sqlx::query_as(
        "SELECT c.primitive_type, c.category, c.schema, c.description
         FROM flint_a2ui.components c
         WHERE c.is_base = true
            OR c.application_id IN (SELECT id FROM flint_a2ui.applications WHERE slug = $1)
         ORDER BY c.category, c.primitive_type",
    )
    .bind(&slug)
    .fetch_all(&state.pool)
    .await
    .map_err(internal_error)?;

    if components.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "catalog not found"})),
        ));
    }

    let mut definitions = serde_json::Map::new();
    for (primitive_type, _category, schema, description) in components {
        let mut def = schema.0.clone();
        if let Some(desc) = description {
            if let Some(obj) = def.as_object_mut() {
                obj.insert("description".to_string(), Value::String(desc));
            }
        }
        definitions.insert(primitive_type, def);
    }

    let catalog = json!({
        "$schema": "https://a2ui.org/schemas/catalog/v0.9.1",
        "catalogId": format!("https://forge.example.com/a2ui/v1/catalog/{slug}/{version}"),
        "name": format!("Flint {} Catalog", slug.replace('-', " ").to_ascii_uppercase()),
        "version": version,
        "definitions": definitions
    });

    Ok::<_, (StatusCode, Json<Value>)>(Json(catalog))
}
