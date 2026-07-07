//! AG-UI Hook Dispatcher — p7-c001 + p7-c002 direct wire.
//!
//! Polls `flint.webhook_outbox` for entries with `target_type = 'agui_run'`
//! and converts them to AG-UI `ToolCallResult` events on the target run's
//! broadcast channel. This is the durable-tier path for hook → AG-UI routing
//! (standard-tier fires directly via `pg_net` in `dispatch_webhook()`).
//!
//! Replaces the FRF agentproto dependency (p7-c002) with a direct in-process
//! wire: no message bus needed, no external service, no new protocol.
//!
//! # Lifecycle
//!
//! Spawned by `main.rs` alongside the webhook outbox processor. Polls every
//! `POLL_INTERVAL` seconds, processes up to `BATCH_SIZE` entries per tick,
//! and marks each entry `delivered` or `failed` using the same retry schedule
//! as `process_webhook_outbox()`.
#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;
use uuid::Uuid;

use crate::routes::agui::AgUiState;
use fdb_domain::AgUiEvent;

const POLL_INTERVAL: Duration = Duration::from_secs(5);
const BATCH_SIZE: i64 = 50;

/// Spawn the AG-UI hook dispatcher background task.
///
/// Returns a `JoinHandle` the caller can abort on graceful shutdown.
pub fn spawn(pool: Arc<PgPool>, agui: AgUiState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            if let Err(e) = process_batch(&pool, &agui).await {
                tracing::warn!(error = %e, "ag-ui hook dispatcher: batch error");
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    })
}

/// Row returned by the batch SELECT.
#[derive(sqlx::FromRow)]
struct OutboxRow {
    id: i64,
    payload: sqlx::types::Json<serde_json::Value>,
    agui_run_id: Option<String>,
    retry_count: i32,
}

/// Process one batch of `agui_run` outbox entries.
async fn process_batch(pool: &PgPool, agui: &AgUiState) -> Result<(), sqlx::Error> {
    let rows: Vec<OutboxRow> = sqlx::query_as(
        "SELECT id, payload, agui_run_id, retry_count
         FROM   flint.webhook_outbox
         WHERE  status      IN ('pending', 'retrying')
           AND  target_type  = 'agui_run'
           AND  visible_at  <= now()
         ORDER  BY visible_at
         LIMIT  $1
         FOR UPDATE SKIP LOCKED",
    )
    .bind(BATCH_SIZE)
    .fetch_all(pool)
    .await?;

    for row in rows {
        match deliver_agui_event(agui, &row).await {
            Ok(()) => {
                sqlx::query(
                    "UPDATE flint.webhook_outbox
                     SET status = 'delivered', updated_at = now()
                     WHERE id = $1",
                )
                .bind(row.id)
                .execute(pool)
                .await?;
            }
            Err(e) => {
                tracing::warn!(id = row.id, error = %e, "ag-ui hook: delivery failed");
                apply_retry(pool, row.id, row.retry_count).await?;
            }
        }
    }

    Ok(())
}

/// Convert an outbox row payload into AG-UI ToolCallResult and publish it.
async fn deliver_agui_event(agui: &AgUiState, row: &OutboxRow) -> Result<(), DeliveryError> {
    let run_id = row
        .agui_run_id
        .as_deref()
        .ok_or(DeliveryError::MissingRunId)?;

    let tool_call_id = Uuid::new_v4().to_string();
    let table = row.payload.get("table").and_then(|v| v.as_str()).unwrap_or("unknown");
    let schema = row.payload.get("schema").and_then(|v| v.as_str()).unwrap_or("public");
    let tool_name = format!("hook:{schema}.{table}");

    // publish() lazily creates the run channel — no need to call channel_for().
    agui.publish(AgUiEvent::ToolCallStart {
        tool_call_id: tool_call_id.clone(),
        tool_name: tool_name.clone(),
        parent_message_id: None,
    })
    .await;

    agui.publish(AgUiEvent::ToolCallResult {
        tool_call_id,
        result: row.payload.0.clone(),
        error: None,
    })
    .await;

    tracing::debug!(run_id, tool_name, "ag-ui hook: ToolCallResult emitted");
    Ok(())
}

/// Apply exponential backoff retry or mark as failed.
async fn apply_retry(pool: &PgPool, id: i64, retry_count: i32) -> Result<(), sqlx::Error> {
    if retry_count >= 4 {
        sqlx::query(
            "UPDATE flint.webhook_outbox
             SET status = 'failed', retry_count = retry_count + 1, updated_at = now()
             WHERE id = $1",
        )
        .bind(id)
        .execute(pool)
        .await?;
    } else {
        let delay_secs: i64 = match retry_count {
            0 => 30,
            1 => 60,
            2 => 120,
            3 => 300,
            _ => 600,
        };
        sqlx::query(
            "UPDATE flint.webhook_outbox
             SET status = 'retrying',
                 retry_count = retry_count + 1,
                 visible_at  = now() + ($1 || ' seconds')::interval,
                 updated_at  = now()
             WHERE id = $2",
        )
        .bind(delay_secs.to_string())
        .bind(id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Errors that can occur during AG-UI event delivery from an outbox entry.
#[derive(Debug, thiserror::Error)]
enum DeliveryError {
    #[error("outbox entry has no agui_run_id")]
    MissingRunId,
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivery_error_displays_clearly() {
        let e = DeliveryError::MissingRunId;
        assert_eq!(e.to_string(), "outbox entry has no agui_run_id");
    }

    #[tokio::test]
    async fn agui_state_channel_is_idempotent() {
        let state = AgUiState::new(8);
        let tx1 = state.channel_for("run-001").await;
        let tx2 = state.channel_for("run-001").await;
        // Same channel returned both times — verify by sending once and receiving twice.
        let mut rx1 = tx1.subscribe();
        let mut rx2 = tx2.subscribe();
        tx1.send(AgUiEvent::RunFinished { run_id: "run-001".into() }).unwrap();
        assert!(rx1.recv().await.is_ok());
        assert!(rx2.recv().await.is_ok());
    }
}
