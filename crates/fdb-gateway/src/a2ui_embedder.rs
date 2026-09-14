//! A2UI embedding generation and recovery, coordinated across gateway replicas.
#![forbid(unsafe_code)]
use sqlx::{Connection, PgConnection, PgPool};
use std::{sync::Arc, time::Duration};

const LOCK_ID: i64 = 0x4132_5549_454d_4244;

/// Model shared by document backfill and semantic queries.
pub fn embedding_model() -> String {
    std::env::var("FLINT_A2UI_EMBED_MODEL").unwrap_or_else(|_| "text-embedding-3-small".into())
}

fn dimensions() -> Result<usize, sqlx::Error> {
    let value = std::env::var("FLINT_A2UI_EMBED_DIMENSIONS").unwrap_or_else(|_| "1536".into());
    value
        .parse::<usize>()
        .ok()
        .filter(|n| *n > 0)
        .ok_or_else(|| sqlx::Error::Protocol("invalid FLINT_A2UI_EMBED_DIMENSIONS".into()))
}

/// Listen before backfilling so inserts during startup are not missed.
/// Periodic recovery also catches notifications lost during reconnection.
pub fn spawn(pool: Arc<PgPool>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut warned = false;
        loop {
            let listener = sqlx::postgres::PgListener::connect_with(&pool).await;
            let Ok(mut listener) = listener else {
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            };
            if listener.listen("a2ui_embed").await.is_err() {
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                let component = tokio::select! {
                    _ = interval.tick() => None,
                    event = listener.recv() => match event {
                        Ok(event) => uuid::Uuid::parse_str(event.payload()).ok(),
                        Err(_) => break,
                    }
                };
                match backfill(&pool, component).await {
                    Ok(()) => {
                        if warned {
                            tracing::info!("a2ui-embedder recovered");
                        }
                        warned = false;
                    }
                    Err(error) => {
                        if !warned {
                            tracing::warn!(%error, "a2ui-embedder unavailable; retrying every 30 seconds");
                        }
                        warned = true;
                    }
                }
            }
        }
    })
}

/// Backfill missing descriptions without duplicating work across replicas.
///
/// # Errors
/// Returns database, configuration, or provider failure after bounded retries.
pub async fn backfill_missing(pool: &PgPool) -> Result<(), sqlx::Error> {
    backfill(pool, None).await
}

async fn backfill(pool: &PgPool, changed: Option<uuid::Uuid>) -> Result<(), sqlx::Error> {
    // Dedicated connection holds a session lock across individual transactions.
    // Explicit close releases the lock even when a database call fails.
    let mut connection = pool.acquire().await?.detach();
    let locked: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
        .bind(LOCK_ID)
        .fetch_one(&mut connection)
        .await?;
    if !locked {
        return Ok(());
    }
    let result = backfill_locked(&mut connection, changed).await;
    let _unlock = sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(LOCK_ID)
        .execute(&mut connection)
        .await;
    result
}

async fn backfill_locked(
    connection: &mut PgConnection,
    changed: Option<uuid::Uuid>,
) -> Result<(), sqlx::Error> {
    let available: bool =
        sqlx::query_scalar("SELECT to_regprocedure('llm.embed(text,text)') IS NOT NULL")
            .fetch_one(&mut *connection)
            .await?;
    if !available {
        return Err(sqlx::Error::Protocol(
            "install the flint_llm extension: llm.embed(text,text) is missing".into(),
        ));
    }
    let expected = dimensions()?;
    let stored: i32 = sqlx::query_scalar("SELECT atttypmod FROM pg_attribute WHERE attrelid='flint_a2ui.embeddings'::regclass AND attname='embedding'")
        .fetch_one(&mut *connection).await?;
    if usize::try_from(stored).ok() != Some(expected) {
        return Err(sqlx::Error::Protocol(
            "catalog vector dimension and configured embedding dimension differ".into(),
        ));
    }
    let model = embedding_model();
    let ids: Vec<uuid::Uuid> = sqlx::query_scalar(
        "SELECT c.id FROM flint_a2ui.components c WHERE NOT EXISTS (
         SELECT 1 FROM flint_a2ui.embeddings e WHERE e.component_id=c.id
         AND e.aspect='description' AND e.model=$1) ORDER BY (c.id=$2) DESC NULLS LAST, c.id",
    )
    .bind(&model)
    .bind(changed)
    .fetch_all(&mut *connection)
    .await?;
    if ids.is_empty() {
        return Ok(());
    }
    let total = ids.len();
    let mut succeeded = 0;
    let mut failed = 0;
    for id in ids {
        let mut attempts = 0;
        loop {
            match embed_component(connection, id, &model, expected).await {
                Ok(()) => {
                    succeeded += 1;
                    break;
                }
                Err(error) if attempts < 2 && retryable(&error) => {
                    tokio::time::sleep(Duration::from_secs(1 << attempts)).await;
                    attempts += 1;
                }
                Err(error) => {
                    failed += 1;
                    tracing::warn!(%error, "a2ui-embedder component failed");
                    // Shared dependency failure: stop the batch rather than
                    // issuing the same failing provider request for every row.
                    tracing::info!(
                        succeeded,
                        failed,
                        remaining = total - succeeded,
                        "a2ui-embedder backfill incomplete"
                    );
                    return Err(error);
                }
            }
        }
    }
    tracing::info!(
        succeeded,
        failed,
        remaining = 0,
        "a2ui-embedder backfill complete"
    );
    Ok(())
}

fn retryable(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Io(_) | sqlx::Error::PoolTimedOut => true,
        sqlx::Error::Database(error) => {
            let message = error.message();
            message.contains("gateway error 429")
                || message.contains("gateway error 5")
                || message.contains("timed out")
                || message.contains("HTTP request failed")
        }
        _ => false,
    }
}

async fn embed_component(
    connection: &mut PgConnection,
    id: uuid::Uuid,
    model: &str,
    expected: usize,
) -> Result<(), sqlx::Error> {
    let mut tx = connection.begin().await?;
    let row: Option<ComponentTextRow> = sqlx::query_as(
        "SELECT slug, primitive_type, category, description, schema, usage_examples FROM flint_a2ui.components WHERE id=$1 FOR SHARE")
        .bind(id).fetch_optional(&mut *tx).await?;
    let Some(row) = row else {
        return Ok(());
    };
    let embedding =
        generate_embedding(&mut tx, &build_embedding_text(&row), model, expected).await?;
    sqlx::query("INSERT INTO flint_a2ui.embeddings(component_id,embedding,entity_type,aspect,model)
        VALUES ($1,$2::vector,'component','description',$3)
        ON CONFLICT (component_id,entity_type,aspect) DO UPDATE SET embedding=EXCLUDED.embedding, model=EXCLUDED.model, created_at=now()")
        .bind(id).bind(embedding).bind(model).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

/// Generate a query vector; failure leaves search on the text path.
pub async fn query_embedding(pool: &PgPool, text: &str) -> Option<String> {
    let mut connection = pool.acquire().await.ok()?;
    if let Ok(vector) = generate_embedding(
        &mut connection,
        text,
        &embedding_model(),
        dimensions().ok()?,
    )
    .await
    {
        Some(vector)
    } else {
        tracing::debug!("semantic query unavailable; using text search");
        None
    }
}

async fn generate_embedding(
    connection: &mut PgConnection,
    text: &str,
    model: &str,
    expected: usize,
) -> Result<String, sqlx::Error> {
    let literal: Option<String> = sqlx::query_scalar("SELECT llm.embed($1,$2)::text")
        .bind(text)
        .bind(model)
        .fetch_one(connection)
        .await?;
    let literal =
        literal.ok_or_else(|| sqlx::Error::Protocol("embedding provider returned NULL".into()))?;
    let vector = parse_vector_literal(&literal)
        .map_err(|_| sqlx::Error::Protocol("invalid embedding vector".into()))?;
    if vector.len() != expected || vector.iter().any(|v| !v.is_finite()) {
        return Err(sqlx::Error::Protocol(
            "embedding dimension mismatch or non-finite values".into(),
        ));
    }
    Ok(vector_literal(&vector))
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
fn parse_vector_literal(s: &str) -> Result<Vec<f32>, VectorError> {
    let trimmed = s.trim();
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .ok_or(VectorError)?;

    if inner.trim().is_empty() {
        return Err(VectorError);
    }

    inner
        .split(',')
        .map(|part| part.trim().parse::<f32>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| VectorError)
}

#[derive(Debug, thiserror::Error)]
#[error("invalid embedding vector")]
struct VectorError;
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
