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

use super::helpers::{claims_json, internal_error};
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

/// `GET /a2ui/v1/components`
///
/// Returns permission-filtered components for the caller. If `app_id` is
/// provided, app-specific components are included when the caller has a role
/// assignment in that application. Base components are always included.
#[tracing::instrument(skip(state, who, query), fields(subject = ?who.role))]
pub async fn list_components(
    State(state): State<A2uiState>,
    Extension(who): Extension<RlsContext>,
    Query(query): Query<ListComponentsQuery>,
) -> impl IntoResponse {
    list_components_value(&state.pool, &who, &query).await
}

/// Inner logic shared with the MCP tool so both surfaces stay in sync.
pub async fn list_components_value(
    pool: &sqlx::PgPool,
    who: &RlsContext,
    query: &ListComponentsQuery,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let claims = claims_json(who);

    let mut components: Vec<ComponentRow> = sqlx::query_as(
        "SELECT id, slug, category, primitive_type, schema, description
         FROM flint_a2ui.resolve_components($1, $2)",
    )
    .bind(query.app_id)
    .bind(claims)
    .fetch_all(pool)
    .await
    .map_err(internal_error)?;

    if let Some(cat) = &query.category {
        components.retain(|c| &c.category == cat);
    }

    Ok(Json(json!({ "components": components })))
}

/// `GET /a2ui/v1/components/{slug}`
///
/// Returns a single component by slug. The caller must be able to see it
/// through `resolve_components` (base components or app-scoped + role).
pub async fn get_component(
    State(state): State<A2uiState>,
    Extension(who): Extension<RlsContext>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    get_component_value(&state.pool, &who, &slug).await
}

/// Inner logic shared with the MCP tool.
pub async fn get_component_value(
    pool: &sqlx::PgPool,
    who: &RlsContext,
    slug: &str,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let claims = claims_json(who);

    let component: Option<ComponentDetailRow> = sqlx::query_as(
        "SELECT c.id, c.slug, c.category, c.primitive_type, c.schema,
                c.description, c.renderers, c.react_pkg, c.flutter_pkg, c.htmx_template
         FROM flint_a2ui.components c
         WHERE c.slug = $1
           AND (
               c.is_base = true
               OR c.application_id IS NULL
               OR c.application_id IN (
                   SELECT DISTINCT ra.application_id
                   FROM flint_a2ui.role_assignments ra
                   WHERE ra.user_id = ($2->'flint'->>'user_id')::text
               )
           )",
    )
    .bind(slug)
    .bind(claims)
    .fetch_optional(pool)
    .await
    .map_err(internal_error)?;

    match component {
        Some(c) => Ok(Json(json!({ "component": c }))),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "component not found"})),
        )),
    }
}

/// `POST /a2ui/v1/components/search`
///
/// Hybrid text + semantic search. When `llm.embed()` is available the query is
/// embedded and `flint_a2ui.hybrid_search()` is used. Otherwise we fall back to
/// a pure full-text search over slug + description.
pub async fn search_components(
    State(state): State<A2uiState>,
    Extension(who): Extension<RlsContext>,
    Json(body): Json<SearchComponentsBody>,
) -> impl IntoResponse {
    search_components_value(&state.pool, &who, &body).await
}

/// Full-text fallback for component search when embeddings are unavailable.
async fn full_text_search(
    pool: &sqlx::PgPool,
    _who: &RlsContext,
    body: &SearchComponentsBody,
    claims: &Value,
) -> Result<Vec<SearchResultRow>, (StatusCode, Json<Value>)> {
    sqlx::query_as(
        "SELECT c.id, c.slug, c.category, c.primitive_type,
                ts_rank(
                    to_tsvector('english', COALESCE(c.description, '') || ' ' || c.slug),
                    plainto_tsquery('english', $1)
                )::double precision AS score
         FROM flint_a2ui.components c
         WHERE (
             c.is_base = true
             OR c.application_id IS NULL
             OR c.application_id = $3
             OR c.application_id IN (
                 SELECT DISTINCT ra.application_id
                 FROM flint_a2ui.role_assignments ra
                 WHERE ra.user_id = ($4->'flint'->>'user_id')::text
             )
         )
         AND to_tsvector('english', COALESCE(c.description, '') || ' ' || c.slug)
             @@ plainto_tsquery('english', $1)
         ORDER BY score DESC
         LIMIT $2",
    )
    .bind(&body.query)
    .bind(body.limit)
    .bind(body.app_id)
    .bind(claims)
    .fetch_all(pool)
    .await
    .map_err(internal_error)
}

/// Inner logic shared with the MCP tool.
pub async fn search_components_value(
    pool: &sqlx::PgPool,
    who: &RlsContext,
    body: &SearchComponentsBody,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let claims = claims_json(who);

    // Prefer hybrid search if an embedding can be generated.
    let hybrid_results: Result<Vec<SearchResultRow>, _> = sqlx::query_as(
        "SELECT c.id, c.slug, c.category, c.primitive_type, hs.score
         FROM flint_a2ui.hybrid_search($1, llm.embed($1), $2) hs
         JOIN flint_a2ui.components c ON c.id = hs.component_id
         WHERE c.is_base = true
            OR c.application_id IS NULL
            OR c.application_id = $3
            OR c.application_id IN (
                SELECT DISTINCT ra.application_id
                FROM flint_a2ui.role_assignments ra
                WHERE ra.user_id = ($4->'flint'->>'user_id')::text
            )
         ORDER BY hs.score DESC
         LIMIT $2",
    )
    .bind(&body.query)
    .bind(body.limit)
    .bind(body.app_id)
    .bind(claims.clone())
    .fetch_all(pool)
    .await;

    // Fall back to full-text search when hybrid is unavailable or returns no
    // results (e.g., the embeddings table is not yet populated).
    let results = match hybrid_results {
        Ok(rows) if !rows.is_empty() => rows,
        Ok(_) => {
            tracing::debug!("hybrid search returned no results; falling back to full-text search");
            full_text_search(pool, who, body, &claims).await?
        }
        Err(e) => {
            tracing::warn!(error = %e, "hybrid search failed; falling back to full-text search");
            full_text_search(pool, who, body, &claims).await?
        }
    };

    Ok(Json(json!({ "results": results })))
}

/// `GET /a2ui/v1/components/bindings/{schema}/{table}`
///
/// Returns auto-generated (and any manual) bindings for a table.
pub async fn get_bindings(
    State(state): State<A2uiState>,
    Path((table_schema, table_name)): Path<(String, String)>,
) -> impl IntoResponse {
    let bindings: Vec<BindingRow> = sqlx::query_as(
        "SELECT b.id, b.table_schema, b.table_name, b.binding_type, b.auto_generated, b.config,
                c.slug, c.primitive_type
         FROM flint_a2ui.bindings b
         JOIN flint_a2ui.components c ON c.id = b.component_id
         WHERE b.table_schema = $1 AND b.table_name = $2",
    )
    .bind(&table_schema)
    .bind(&table_name)
    .fetch_all(&state.pool)
    .await
    .map_err(internal_error)?;

    Ok::<_, (StatusCode, Json<Value>)>(Json(json!({ "bindings": bindings })))
}
