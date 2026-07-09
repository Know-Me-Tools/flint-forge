//! A2UI Registry REST API routes.
//!
//! All routes under `/a2ui/v1/*` are protected by JWT authentication via the
//! `require_rls` middleware. They use a privileged `PgPool` from
//! `GatewayState` to call SECURITY DEFINER functions and read catalog data.
//!
//! # Endpoints
//!
//! - `GET    /a2ui/v1/components`
//! - `GET    /a2ui/v1/components/{slug}`
//! - `POST   /a2ui/v1/components/search`
//! - `GET    /a2ui/v1/components/bindings/{schema}/{table}`
//! - `GET    /a2ui/v1/applications`
//! - `GET    /a2ui/v1/applications/{id}`
//! - `GET    /a2ui/v1/catalog/{*catalog_id}`
//! - `POST   /a2ui/v1/surfaces/assemble`
#![forbid(unsafe_code)]

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    Extension,
};
use fdb_reflection::compilers::a2ui::{A2uiAssembler, AssemblerError, AssemblyContext};
use forge_identity::RlsContext;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{types::Json as SqlxJson, FromRow};
use uuid::Uuid;

use crate::GatewayState;

/// Route-scoped state for A2UI handlers. It intentionally exposes only the
/// privileged pool so tests and the production composition root can construct
/// it without building GraphQL/vector executors.
#[derive(Clone)]
pub struct A2uiState {
    pub pool: sqlx::PgPool,
}

impl From<GatewayState> for A2uiState {
    fn from(state: GatewayState) -> Self {
        Self { pool: state.pool }
    }
}

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

/// JSON body for `POST /a2ui/v1/surfaces/assemble`.
#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct AssembleSurfaceBody {
    pub event_type: String,
    #[serde(default)]
    pub event_context: Value,
    #[serde(default)]
    pub application_id: Option<Uuid>,
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

/// Application row.
#[derive(Debug, Serialize, FromRow)]
struct ApplicationRow {
    id: Uuid,
    slug: String,
    name: String,
    description: Option<String>,
    jwt_claims_template: SqlxJson<Value>,
    catalog_id: Option<String>,
    is_system: bool,
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

// ─── Components ─────────────────────────────────────────────────────────────

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
                ) AS score
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

// ─── Applications ───────────────────────────────────────────────────────────

/// `GET /a2ui/v1/applications`
///
/// Lists applications the caller has access to. System applications are always
/// visible; non-system applications require a role assignment.
pub async fn list_applications(
    State(state): State<A2uiState>,
    Extension(who): Extension<RlsContext>,
) -> impl IntoResponse {
    let user_id = user_id_from_claims(&who);

    let apps: Vec<ApplicationRow> = sqlx::query_as(
        "SELECT a.id, a.slug, a.name, a.description, a.jwt_claims_template, a.catalog_id, a.is_system
         FROM flint_a2ui.applications a
         WHERE a.is_system = true
            OR $1::text IS NULL
            OR a.id IN (
                SELECT DISTINCT application_id FROM flint_a2ui.role_assignments
                WHERE user_id = $1
            )
         ORDER BY a.is_system DESC, a.slug",
    )
    .bind(user_id.as_deref())
    .fetch_all(&state.pool)
    .await
    .map_err(internal_error)?;

    Ok::<_, (StatusCode, Json<Value>)>(Json(json!({ "applications": apps })))
}

/// `GET /a2ui/v1/applications/{id}`
///
/// Returns a single application.
pub async fn get_application(
    State(state): State<A2uiState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let app: Option<ApplicationRow> = sqlx::query_as(
        "SELECT id, slug, name, description, jwt_claims_template, catalog_id, is_system
         FROM flint_a2ui.applications WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?;

    match app {
        Some(a) => Ok(Json(json!({ "application": a }))),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "application not found"})),
        )),
    }
}

// ─── Design Systems ─────────────────────────────────────────────────────────

/// `GET /a2ui/v1/design-systems/{id}/tokens`
///
/// Returns the design system's tokens in W3C Design Token format.
/// The tokens are stored as jsonb in `flint_a2ui.design_systems.tokens`.
pub async fn get_design_system_tokens(
    State(state): State<A2uiState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let row: Option<(SqlxJson<Value>,)> =
        sqlx::query_as("SELECT tokens FROM flint_a2ui.design_systems WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.pool)
            .await
            .map_err(internal_error)?;

    match row {
        Some((tokens,)) => Ok(Json(tokens.0)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "design system not found"})),
        )),
    }
}

// ─── Catalog ────────────────────────────────────────────────────────────────

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

// ─── Surfaces ─────────────────────────────────────────────────────────────────

/// `POST /a2ui/v1/surfaces/assemble`
///
/// Assembles an A2UI surface from an event context. Delegates to
/// `A2uiAssembler` in `fdb-reflection`, which applies application-specific
/// assembly rules and falls back to default table bindings.
#[tracing::instrument(skip(state, who, body), fields(event_type = %body.event_type))]
pub async fn assemble_surface(
    State(state): State<A2uiState>,
    Extension(who): Extension<RlsContext>,
    Json(body): Json<AssembleSurfaceBody>,
) -> impl IntoResponse {
    assemble_surface_value(&state.pool, &who, &body).await
}

/// Inner logic shared with the MCP tool.
pub async fn assemble_surface_value(
    pool: &sqlx::PgPool,
    who: &RlsContext,
    body: &AssembleSurfaceBody,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let ctx = AssemblyContext {
        event_type: body.event_type.clone(),
        event_payload: body.event_context.clone(),
        application_id: body.application_id,
        jwt_claims: claims_json(who),
        surface_id: None,
    };

    let assembler = A2uiAssembler::new(pool.clone());
    match assembler.assemble(&ctx).await {
        Ok(surface) => Ok(Json(surface.to_json())),
        Err(err) => Err(assembler_error(err)),
    }
}

// ─── helpers ──────────────────────────────────────────────────────────────────

/// Extract `flint.user_id` from the `RlsContext` claims string.
fn user_id_from_claims(who: &RlsContext) -> Option<String> {
    claims_json(who)
        .get("flint")
        .and_then(|v| v.get("user_id"))
        .and_then(|v| v.as_str())
        .map(String::from)
}

/// Build a JSON object from the `RlsContext` claims string.
fn claims_json(who: &RlsContext) -> Value {
    serde_json::from_str(&who.claims_json).unwrap_or(Value::Null)
}

fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, Json<Value>) {
    tracing::error!(error = %err, "a2ui api error");
    // Temporary diagnostic for CI-gated DB tests; remove once integration suite
    // is stable.
    eprintln!("a2ui internal_error: {err}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "internal server error" })),
    )
}

fn assembler_error(err: AssemblerError) -> (StatusCode, Json<Value>) {
    match err {
        AssemblerError::NoBinding(schema, table) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "no binding",
                "schema": schema,
                "table": table,
            })),
        ),
        AssemblerError::MissingField(field) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "missing field",
                "field": field,
            })),
        ),
        AssemblerError::InvalidConfig(msg) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "invalid config",
                "message": msg,
            })),
        ),
        AssemblerError::Database(e) => {
            tracing::error!(error = %e, "a2ui assembler database error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "internal server error" })),
            )
        }
        _ => {
            tracing::error!(error = %err, "a2ui assembler error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "internal server error" })),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        extract::Request,
        http::{Method, StatusCode},
        routing::{get, post},
        Router,
    };
    use serde_json::json;
    use sqlx::PgPool;
    use tower::ServiceExt;

    async fn connect() -> Option<(PgPool, A2uiState)> {
        let url = std::env::var("DATABASE_URL").ok()?;
        let pool = PgPool::connect(&url).await.ok()?;
        let state = A2uiState { pool: pool.clone() };
        Some((pool, state))
    }

    fn fake_rls_context(user_id: &str) -> RlsContext {
        RlsContext {
            role: "authenticated".to_string(),
            claims_json: json!({"flint": {"user_id": user_id}}).to_string(),
            raw_bearer: "fake".to_string(),
            keto_subject: user_id.to_string(),
            vault_key_id: None,
        }
    }

    fn a2ui_app(state: A2uiState, user_id: &str) -> Router {
        Router::new()
            .route("/a2ui/v1/components", get(list_components))
            .route("/a2ui/v1/components/search", post(search_components))
            .route(
                "/a2ui/v1/components/bindings/{schema}/{table}",
                get(get_bindings),
            )
            .route("/a2ui/v1/components/{slug}", get(get_component))
            .route("/a2ui/v1/applications", get(list_applications))
            .route("/a2ui/v1/applications/{id}", get(get_application))
            .route("/a2ui/v1/catalog/{*catalog_id}", get(get_catalog))
            .route("/a2ui/v1/surfaces/assemble", post(assemble_surface))
            .route(
                "/a2ui/v1/design-systems/{id}/tokens",
                get(get_design_system_tokens),
            )
            .layer(Extension(fake_rls_context(user_id)))
            .with_state(state)
    }

    async fn read_json_body(resp: axum::response::Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("read body");
        serde_json::from_slice(&bytes).expect("body is valid JSON")
    }

    #[tokio::test]
    async fn test_list_components_returns_base_components() {
        let Some((_pool, state)) = connect().await else {
            return;
        };
        let app = a2ui_app(state, "anonymous-user");

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/a2ui/v1/components")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request");

        assert_eq!(resp.status(), StatusCode::OK);
        let body = read_json_body(resp).await;
        let components = body["components"].as_array().expect("components array");
        let slugs: Vec<String> = components
            .iter()
            .filter_map(|c| c["slug"].as_str().map(String::from))
            .collect();
        assert!(slugs.contains(&"data-grid".to_string()));
        assert!(slugs.contains(&"button".to_string()));
    }

    #[tokio::test]
    async fn test_get_component_returns_schema() {
        let Some((_pool, state)) = connect().await else {
            return;
        };
        let app = a2ui_app(state, "anonymous-user");

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/a2ui/v1/components/button")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request");

        assert_eq!(resp.status(), StatusCode::OK);
        let body = read_json_body(resp).await;
        assert_eq!(body["component"]["slug"], "button");
        assert!(body["component"]["schema"].is_object());
    }

    #[tokio::test]
    async fn test_search_components_finds_button() {
        let Some((_pool, state)) = connect().await else {
            return;
        };
        let app = a2ui_app(state, "anonymous-user");

        let req = Request::builder()
            .method(Method::POST)
            .uri("/a2ui/v1/components/search")
            .header("content-type", "application/json")
            .body(Body::from(json!({"query": "button"}).to_string()))
            .unwrap();

        let resp = app.oneshot(req).await.expect("request");
        assert_eq!(resp.status(), StatusCode::OK);
        let body = read_json_body(resp).await;
        let results = body["results"].as_array().expect("results array");
        assert!(
            !results.is_empty(),
            "search should return at least one result"
        );
    }

    #[tokio::test]
    async fn test_get_catalog_returns_json_schema() {
        let Some((_pool, state)) = connect().await else {
            return;
        };
        let app = a2ui_app(state, "anonymous-user");

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/a2ui/v1/catalog/flint-base/1.0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request");

        assert_eq!(resp.status(), StatusCode::OK);
        let body = read_json_body(resp).await;
        assert_eq!(body["$schema"], "https://a2ui.org/schemas/catalog/v0.9.1");
        assert!(
            body["definitions"]["Button"].is_object() || body["definitions"]["button"].is_object()
        );
    }

    #[tokio::test]
    async fn test_assemble_surface_validates_input() {
        let Some((_pool, state)) = connect().await else {
            return;
        };
        let app = a2ui_app(state, "anonymous-user");

        let req = Request::builder()
            .method(Method::POST)
            .uri("/a2ui/v1/surfaces/assemble")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&AssembleSurfaceBody {
                    event_type: "mount".to_string(),
                    event_context: json!({}),
                    application_id: None,
                })
                .unwrap(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.expect("request");
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_get_design_system_tokens_not_found() {
        let Some((_pool, state)) = connect().await else {
            return;
        };
        let app = a2ui_app(state, "anonymous-user");
        let id = Uuid::new_v4();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/a2ui/v1/design-systems/{id}/tokens"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("req");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
