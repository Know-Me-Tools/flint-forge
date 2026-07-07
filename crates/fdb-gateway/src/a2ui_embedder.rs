//! A2UI component embedder — background task for the Flint A2UI Component Registry.
//!
//! Listens on the Postgres `a2ui_embed` channel for new component inserts, calls
//! the in-database `llm.embed()` function via the privileged reflection pool, and
//! writes the resulting `vector(1536)` into `flint_a2ui.embeddings`.
//!
//! On startup the task also backfills any components that lack an embedding row.
//!
//! # Security invariants
//!
//! - Runs as a **privileged Postgres role** (service account, not `authenticated`).
//!   The pool used here MUST be configured with a superuser or service-role credential.
//! - Component text (slug, description, prop names) is NOT PII, but is still treated
//!   as internal catalog data and logged only at `debug` level.
//! - If `llm.embed()` is unavailable (e.g. `ext-flint-llm` not installed), the task
//!   logs a warning and continues; semantic search degrades gracefully to text search.
//!
//! # OQ-10 resolution
//!
//! The default model is `text-embedding-3-large`. If that model is unavailable via
//! the liter-llm gateway, the task falls back to `text-embedding-3-small` (also
//! 1536-d) automatically. If both fail, the component is left unembedded and will
//! be retried on the next startup or insert.
#![forbid(unsafe_code)]

use std::sync::Arc;

use sqlx::PgPool;
use tracing::instrument;

/// Spawn the A2UI embedder background task.
///
/// The task:
/// 1. Runs an initial backfill for components without embeddings.
/// 2. Opens a `PgListener` on the `a2ui_embed` channel.
/// 3. Processes each notification by embedding the referenced component.
///
/// The task never panics; errors are logged and the loop continues.
pub fn spawn(pool: Arc<PgPool>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(e) = backfill_missing(&pool).await {
            tracing::warn!(error = %e, "a2ui-embedder initial backfill failed");
        }

        let mut listener = match sqlx::postgres::PgListener::connect_with(&pool).await {
            Ok(l) => l,
            Err(e) => {
                tracing::error!(error = %e, "a2ui-embedder failed to connect listener");
                return;
            }
        };

        if let Err(e) = listener.listen("a2ui_embed").await {
            tracing::error!(error = %e, "a2ui-embedder failed to LISTEN a2ui_embed");
            return;
        }

        tracing::info!("a2ui-embedder listening on a2ui_embed");

        loop {
            match listener.recv().await {
                Ok(notification) => {
                    let payload = notification.payload();
                    tracing::debug!(payload, "a2ui_embed notification received");
                    if let Ok(id) = uuid::Uuid::parse_str(payload) {
                        if let Err(e) = embed_component(&pool, id).await {
                            tracing::warn!(error = %e, component_id = %id, "a2ui-embedder failed to embed component");
                        }
                    } else {
                        tracing::warn!(payload, "a2ui_embed notification payload is not a UUID");
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "a2ui-embedder listener recv failed; reconnecting");
                    // Best-effort reconnect. If reconnect fails, exit and let the
                    // operator restart; otherwise continue listening.
                    listener = match reconnect(&pool).await {
                        Some(l) => l,
                        None => return,
                    };
                }
            }
        }
    })
}

/// Re-establish a listener connection and re-subscribe.
async fn reconnect(pool: &PgPool) -> Option<sqlx::postgres::PgListener> {
    let mut listener = match sqlx::postgres::PgListener::connect_with(pool).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(error = %e, "a2ui-embedder reconnect failed");
            return None;
        }
    };
    if let Err(e) = listener.listen("a2ui_embed").await {
        tracing::error!(error = %e, "a2ui-embedder reconnect LISTEN failed");
        return None;
    }
    Some(listener)
}

/// Embed all components that do not yet have an `aspect = 'description'` embedding.
#[instrument(skip(pool))]
pub async fn backfill_missing(pool: &PgPool) -> Result<(), sqlx::Error> {
    let rows: Vec<(uuid::Uuid,)> = sqlx::query_as(
        "SELECT c.id
         FROM flint_a2ui.components c
         WHERE NOT EXISTS (
             SELECT 1 FROM flint_a2ui.embeddings e
             WHERE e.component_id = c.id AND e.aspect = 'description'
         )",
    )
    .fetch_all(pool)
    .await?;

    tracing::info!(count = rows.len(), "a2ui-embedder backfill starting");

    for (id,) in rows {
        if let Err(e) = embed_component(pool, id).await {
            tracing::warn!(error = %e, component_id = %id, "a2ui-embedder backfill item failed");
        }
    }

    tracing::info!("a2ui-embedder backfill complete");
    Ok(())
}

/// Fetch component text, generate an embedding, and insert it into
/// `flint_a2ui.embeddings`.
#[instrument(skip(pool), fields(component_id = %id))]
async fn embed_component(pool: &PgPool, id: uuid::Uuid) -> Result<(), EmbedError> {
    let row: ComponentTextRow = sqlx::query_as(
        "SELECT slug, primitive_type, category, description, schema, usage_examples
         FROM flint_a2ui.components
         WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(EmbedError::ComponentNotFound(id))?;

    let text = build_embedding_text(&row);
    tracing::debug!(text_len = text.len(), "a2ui-embedder built embedding text");

    let (embedding, model) = match generate_embedding(pool, &text, "text-embedding-3-large").await {
        Ok(v) => (v, "text-embedding-3-large"),
        Err(EmbedError::ModelUnavailable(_)) => {
            tracing::info!(
                "text-embedding-3-large unavailable; falling back to text-embedding-3-small"
            );
            (
                generate_embedding(pool, &text, "text-embedding-3-small").await?,
                "text-embedding-3-small",
            )
        }
        Err(e) => return Err(e),
    };

    sqlx::query(
        "INSERT INTO flint_a2ui.embeddings
             (component_id, embedding, entity_type, aspect, model)
         VALUES ($1, $2::vector(1536), 'component', 'description', $3)
         ON CONFLICT (component_id, entity_type, aspect)
         DO UPDATE SET embedding = EXCLUDED.embedding,
                       model = EXCLUDED.model,
                       created_at = now()",
    )
    .bind(id)
    .bind(vector_literal(&embedding))
    .bind(model)
    .execute(pool)
    .await?;

    Ok(())
}

/// Row type for the component text used to build an embedding input.
#[derive(sqlx::FromRow)]
struct ComponentTextRow {
    slug: String,
    primitive_type: String,
    category: String,
    description: Option<String>,
    schema: sqlx::types::Json<serde_json::Value>,
    usage_examples: Option<sqlx::types::Json<serde_json::Value>>,
}

/// Build the embedding input string from component metadata.
fn build_embedding_text(row: &ComponentTextRow) -> String {
    let mut parts = vec![
        row.slug.clone(),
        row.primitive_type.clone(),
        row.category.clone(),
    ];

    if let Some(desc) = &row.description {
        parts.push(desc.clone());
    }

    parts.push("Usage:".to_string());
    if let Some(examples) = &row.usage_examples {
        parts.push(serde_json::to_string(&examples.0).unwrap_or_default());
    }

    parts.push("Props:".to_string());
    if let Some(props) = row.schema.0.get("properties").and_then(|v| v.as_object()) {
        for key in props.keys() {
            parts.push(key.clone());
        }
    }

    parts.join(" ")
}

/// Call the in-database `llm.embed(text, model)` function and return a `Vec<f32>`.
#[instrument(skip(pool))]
async fn generate_embedding(
    pool: &PgPool,
    text: &str,
    model: &str,
) -> Result<Vec<f32>, EmbedError> {
    let vector_literal: Option<String> = sqlx::query_scalar("SELECT llm.embed($1, $2)::text")
        .bind(text)
        .bind(model)
        .fetch_optional(pool)
        .await?;

    let vector_literal =
        vector_literal.ok_or_else(|| EmbedError::ModelUnavailable(model.to_string()))?;
    parse_vector_literal(&vector_literal)
}

/// Format a `Vec<f32>` as a Postgres `vector` literal string.
fn vector_literal(v: &[f32]) -> String {
    let joined = v
        .iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{joined}]")
}

/// Parse a Postgres `vector` text representation such as `[0.1, -0.2, ...]`.
fn parse_vector_literal(s: &str) -> Result<Vec<f32>, EmbedError> {
    let trimmed = s.trim();
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .ok_or_else(|| EmbedError::VectorParse(s.to_string()))?;

    if inner.trim().is_empty() {
        return Err(EmbedError::VectorParse(s.to_string()));
    }

    inner
        .split(',')
        .map(|part| part.trim().parse::<f32>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| EmbedError::VectorParse(s.to_string()))
}

/// Errors that can occur while embedding a component.
#[derive(Debug, thiserror::Error)]
enum EmbedError {
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("component not found: {0}")]
    ComponentNotFound(uuid::Uuid),

    #[error("embedding model unavailable: {0}")]
    ModelUnavailable(String),

    #[error("failed to parse vector literal: {0}")]
    VectorParse(String),
}

// ─── unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_vector_literal_parses_bracketed_floats() {
        let parsed = parse_vector_literal("[0.1, -0.2, 0.3]").unwrap();
        assert_eq!(parsed, vec![0.1, -0.2, 0.3]);
    }

    #[test]
    fn parse_vector_literal_rejects_malformed() {
        assert!(parse_vector_literal("not-a-vector").is_err());
        assert!(parse_vector_literal("[]").is_err());
        assert!(parse_vector_literal("1.0, 2.0").is_err());
    }

    #[test]
    fn build_embedding_text_includes_props() {
        let row = ComponentTextRow {
            slug: "text-input".into(),
            primitive_type: "TextInput".into(),
            category: "input".into(),
            description: Some("A text input field".into()),
            schema: sqlx::types::Json(serde_json::json!({
                "properties": {
                    "label": { "type": "string" },
                    "placeholder": { "type": "string" }
                }
            })),
            usage_examples: None,
        };
        let text = build_embedding_text(&row);
        assert!(text.contains("text-input"));
        assert!(text.contains("A text input field"));
        assert!(text.contains("label"));
        assert!(text.contains("placeholder"));
    }
}
