//! Component listing, detail, search, and binding lookup handlers.
//!
//! - `GET    /a2ui/v1/components`
//! - `GET    /a2ui/v1/components/{slug}`
//! - `POST   /a2ui/v1/components/search`
//! - `GET    /a2ui/v1/components/bindings/{schema}/{table}`

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    Extension,
};
use forge_identity::RlsContext;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{types::Json as SqlxJson, FromRow};
use uuid::Uuid;

use super::helpers::{internal_error, transaction};
use super::A2uiState;

/// Query parameters for `GET /a2ui/v1/components`.
#[derive(Debug, Deserialize)]
pub struct ListComponentsQuery {
    /// Filter to an application catalog. When omitted, only base components are
    /// returned by `flint_a2ui.resolve_components`.
    #[serde(default)]
    pub app_id: Option<Uuid>,
    /// Optional category filter applied after SQL resolution.
    #[serde(default)]
    pub category: Option<String>,
}

/// JSON body for `POST /a2ui/v1/components/search`.
#[derive(Debug, Deserialize)]
pub struct SearchComponentsBody {
    pub query: String,
    #[serde(default = "default_search_limit")]
    pub limit: i32,
    #[serde(default)]
    pub app_id: Option<Uuid>,
}

fn default_search_limit() -> i32 {
    10
}

/// Component row returned by `resolve_components`.
#[derive(Debug, Serialize, FromRow)]
struct ComponentRow {
    id: Uuid,
    slug: String,
    category: String,
    primitive_type: String,
    schema: SqlxJson<Value>,
    description: Option<String>,
}

/// Component detail row.
#[derive(Debug, Serialize, FromRow)]
struct ComponentDetailRow {
    id: Uuid,
    slug: String,
    category: String,
    primitive_type: String,
    schema: SqlxJson<Value>,
    description: Option<String>,
    renderers: SqlxJson<Value>,
    react_pkg: Option<String>,
    flutter_pkg: Option<String>,
    htmx_template: Option<String>,
}

/// Binding row.
#[derive(Debug, Serialize, FromRow)]
struct BindingRow {
    id: Uuid,
    table_schema: String,
    table_name: String,
    binding_type: String,
    auto_generated: bool,
    config: SqlxJson<Value>,
    slug: String,
    primitive_type: String,
}

/// Search result row.
#[derive(Debug, Serialize, FromRow)]
struct SearchResultRow {
    id: Uuid,
    slug: String,
    category: String,
    primitive_type: String,
    score: f64,
}

/// List catalog components under the caller's RLS identity.
pub async fn list_components(
    State(state): State<A2uiState>,
    Extension(who): Extension<RlsContext>,
    Query(query): Query<ListComponentsQuery>,
) -> impl IntoResponse {
    list_components_value(&state.pool, &who, &query).await
}

/// Shared by REST and MCP.
pub async fn list_components_value(
    pool: &sqlx::PgPool,
    who: &RlsContext,
    query: &ListComponentsQuery,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut tx = transaction(pool, who).await?;
    let components: Vec<ComponentRow> = sqlx::query_as(
        "SELECT id, slug, category, primitive_type, schema, description FROM flint_a2ui.components
         WHERE (is_base OR application_id IS NULL OR application_id = $1)
         AND ($2::text IS NULL OR category = $2) ORDER BY category, slug",
    )
    .bind(query.app_id)
    .bind(&query.category)
    .fetch_all(&mut *tx)
    .await
    .map_err(internal_error)?;
    Ok(Json(json!({"components": components})))
}

/// Fetch one visible component, or return 404.
pub async fn get_component(
    State(state): State<A2uiState>,
    Extension(who): Extension<RlsContext>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    get_component_value(&state.pool, &who, &slug).await
}

/// Shared by REST and MCP.
pub async fn get_component_value(
    pool: &sqlx::PgPool,
    who: &RlsContext,
    slug: &str,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut tx = transaction(pool, who).await?;
    let component: Option<ComponentDetailRow> = sqlx::query_as(
        "SELECT id, slug, category, primitive_type, schema, description, renderers,
         react_pkg, flutter_pkg, htmx_template FROM flint_a2ui.components WHERE slug = $1",
    )
    .bind(slug)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal_error)?;
    component.map(|c| Json(json!({"component": c}))).ok_or((
        StatusCode::NOT_FOUND,
        Json(json!({"error":"component not found"})),
    ))
}

/// Search the catalog; provider outages fall back to full-text search.
pub async fn search_components(
    State(state): State<A2uiState>,
    Extension(who): Extension<RlsContext>,
    Json(body): Json<SearchComponentsBody>,
) -> impl IntoResponse {
    search_components_value(&state.pool, &who, &body).await
}

/// Shared by REST and MCP. Embed separately so a provider SQL error cannot
/// abort the subsequent RLS search transaction.
pub async fn search_components_value(
    pool: &sqlx::PgPool,
    who: &RlsContext,
    body: &SearchComponentsBody,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !(1..=100).contains(&body.limit) || body.query.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"query is required; limit must be 1..100"})),
        ));
    }
    let vector = fdb_gateway::a2ui_embedder::query_embedding(pool, &body.query).await;
    let mut tx = transaction(pool, who).await?;
    let mut results: Vec<SearchResultRow> = Vec::new();
    if let Some(vector) = vector {
        // Visibility is filtered BEFORE ranking and LIMIT, including MCP calls.
        results = sqlx::query_as(
            "SELECT c.id, c.slug, c.category, c.primitive_type,
              (0.7 * (1 - (e.embedding <=> $1::vector)) + 0.3 * ts_rank(
                to_tsvector('english', COALESCE(c.description,'') || ' ' || c.slug),
                plainto_tsquery('english', $2)))::double precision AS score
             FROM flint_a2ui.components c JOIN flint_a2ui.embeddings e ON e.component_id=c.id
             WHERE e.aspect='description' AND e.model=$5
               AND ($3::uuid IS NULL OR c.is_base OR c.application_id IS NULL OR c.application_id=$3)
             ORDER BY score DESC LIMIT $4")
            .bind(vector).bind(&body.query).bind(body.app_id).bind(body.limit)
            .bind(fdb_gateway::a2ui_embedder::embedding_model()).fetch_all(&mut *tx).await.map_err(internal_error)?;
    }
    if results.is_empty() {
        results = sqlx::query_as(
            "SELECT id, slug, category, primitive_type, ts_rank(
                to_tsvector('english', COALESCE(description,'') || ' ' || slug),
                plainto_tsquery('english', $1))::double precision AS score
             FROM flint_a2ui.components
             WHERE ($2::uuid IS NULL OR is_base OR application_id IS NULL OR application_id=$2)
               AND to_tsvector('english', COALESCE(description,'') || ' ' || slug) @@ plainto_tsquery('english', $1)
             ORDER BY score DESC LIMIT $3")
            .bind(&body.query).bind(body.app_id).bind(body.limit).fetch_all(&mut *tx).await.map_err(internal_error)?;
    }
    Ok(Json(json!({"results": results})))
}

/// Return visible bindings for a table.
pub async fn get_bindings(
    State(state): State<A2uiState>,
    Extension(who): Extension<RlsContext>,
    Path((schema, table)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut tx = transaction(&state.pool, &who).await?;
    let bindings: Vec<BindingRow> = sqlx::query_as(
        "SELECT b.id, b.table_schema, b.table_name, b.binding_type, b.auto_generated, b.config,
                c.slug, c.primitive_type FROM flint_a2ui.bindings b
         JOIN flint_a2ui.components c ON c.id=b.component_id WHERE b.table_schema=$1 AND b.table_name=$2")
        .bind(schema).bind(table).fetch_all(&mut *tx).await.map_err(internal_error)?;
    Ok(Json(json!({"bindings":bindings})))
}
