use std::sync::Arc;

use fdb_domain::ChangeEvent;

use super::error::{redact, ListenError};
use super::listen_loop::listen_loop;
use super::{ListenConfig, CHANNEL};
use crate::KetoConfig;

/// `ChangeStreamSource` backed by in-process Postgres `LISTEN`/`NOTIFY`.
///
/// One instance is shared across all subscription connections. A single background
/// task owns the dedicated `PgListener` and fans decoded events out over a
/// `tokio::sync::broadcast`; each `watch()` call subscribes an independent receiver
/// and filters it by `entity_type`.
#[derive(Clone)]
pub struct ListenChangeSource {
    /// Sender side of the fan-out; every `watch()` subscribes a receiver.
    /// `ChangeEvent` is `Arc`-wrapped so the single decode is shared across N
    /// subscribers without per-subscriber clones of a potentially large record.
    pub(super) tx: tokio::sync::broadcast::Sender<Arc<ChangeEvent>>,
    /// HTTP client for the Keto coarse check.
    pub(super) http: reqwest::Client,
    /// Keto read-API config (reused from the crate root).
    pub(super) keto: KetoConfig,
    /// Aborts the background listen task (and drops its dedicated PG connection)
    /// when the last clone of this source is dropped. Shared via `Arc` so cloning
    /// the source keeps the single task alive; the guard fires only on final drop.
    _task: Arc<ListenTaskGuard>,
}

/// Owns the background listen task's abort handle; aborts on drop so a dropped
/// `ListenChangeSource` (e.g. reconfigure, test teardown) does not leak the task
/// and its dedicated Postgres connection.
struct ListenTaskGuard(tokio::task::AbortHandle);

impl Drop for ListenTaskGuard {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl ListenChangeSource {
    /// Connect the dedicated `PgListener`, `LISTEN` on `flint_change`, and spawn the
    /// single background task that decodes notifications into the broadcast.
    ///
    /// Fails closed at construction: if the listener cannot connect or `LISTEN`,
    /// returns `Err` — the gateway MUST NOT mount subscriptions without a live feed.
    ///
    /// # Errors
    ///
    /// - [`ListenError::Connect`] if the dedicated connection cannot be established.
    /// - [`ListenError::Listen`] if `LISTEN flint_change` fails.
    pub async fn new(cfg: ListenConfig, keto: KetoConfig) -> Result<Self, ListenError> {
        let mut listener = sqlx::postgres::PgListener::connect(&cfg.database_url)
            .await
            .map_err(|e| {
                // Never surface the DSN (may contain credentials). Log without it.
                tracing::error!("listener connect failed: {}", redact(&e));
                ListenError::Connect
            })?;
        listener.listen(CHANNEL).await.map_err(|e| {
            tracing::error!(channel = CHANNEL, "listen setup failed: {}", redact(&e));
            ListenError::Listen
        })?;

        let capacity = cfg.broadcast_capacity.max(1);
        let (tx, _rx) = tokio::sync::broadcast::channel(capacity);
        let task_tx = tx.clone();
        let handle = tokio::spawn(listen_loop(listener, task_tx));

        Ok(Self {
            tx,
            http: reqwest::Client::new(),
            keto,
            _task: Arc::new(ListenTaskGuard(handle.abort_handle())),
        })
    }
}
