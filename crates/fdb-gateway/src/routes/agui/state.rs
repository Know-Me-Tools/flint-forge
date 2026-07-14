//! `AgUiState` — shared per-run broadcast channel registry.

use std::collections::HashMap;
use std::sync::Arc;

use futures::stream::{self, Stream};
use tokio::sync::{broadcast, Mutex};

use fdb_domain::AgUiEvent;

/// Shared AG-UI state — holds per-run broadcast channels.
#[derive(Clone)]
pub struct AgUiState {
    pub(super) inner: Arc<AgUiInner>,
    /// Privileged PgPool for A2UI surface assembly.
    /// When present, enables `POST /agents/v1/:run_id/surfaces/assemble`.
    pub a2ui_pool: Option<sqlx::PgPool>,
}

pub(super) struct AgUiInner {
    /// Map of run_id → broadcast sender. Created lazily on first publish or subscribe.
    pub(super) runs: Mutex<HashMap<String, broadcast::Sender<AgUiEvent>>>,
    /// Channel capacity for each run's broadcast channel.
    capacity: usize,
}

impl AgUiState {
    /// Create a new AG-UI state with the given broadcast capacity per run.
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(AgUiInner {
                runs: Mutex::new(HashMap::new()),
                capacity,
            }),
            a2ui_pool: None,
        }
    }

    /// Attach a privileged pool for A2UI assembly.
    pub fn with_pool(mut self, pool: sqlx::PgPool) -> Self {
        self.a2ui_pool = Some(pool);
        self
    }

    /// Get or create the broadcast channel for a run.
    pub(crate) async fn channel_for(&self, run_id: &str) -> broadcast::Sender<AgUiEvent> {
        let mut runs = self.inner.runs.lock().await;
        if let Some(tx) = runs.get(run_id) {
            tx.clone()
        } else {
            let (tx, _rx) = broadcast::channel(self.inner.capacity);
            runs.insert(run_id.to_owned(), tx.clone());
            tx
        }
    }

    /// Publish an event to the run's broadcast channel.
    /// Returns `Ok(())` even if there are no subscribers (event is dropped).
    pub async fn publish(&self, event: AgUiEvent) {
        let run_id = match event.run_id() {
            Some(id) => id.to_owned(),
            None => return,
        };
        let tx = self.channel_for(&run_id).await;
        let _ = tx.send(event);
    }

    /// Broadcast an event to ALL connected AG-UI runs.
    /// Used for system-wide notifications like schema version changes that
    /// every live session must observe regardless of which run it is in.
    /// Bypasses `run_id()` routing — the same event is sent to every channel.
    // p14-c003: A2UI catalog hot-reload — when the StateManager hot-swaps the
    // compiled state on `meta_runtime` NOTIFY, this fan-outs the new version
    // to every SSE subscriber so SDKs can revalidate their registry.
    pub async fn broadcast_all(&self, event: AgUiEvent) {
        let runs = self.inner.runs.lock().await;
        for sender in runs.values() {
            let _ = sender.send(event.clone());
        }
    }

    /// Subscribe to a run's event stream. Returns `None` if the run doesn't exist.
    pub async fn subscribe(&self, run_id: &str) -> Option<impl Stream<Item = AgUiEvent>> {
        let tx = {
            let runs = self.inner.runs.lock().await;
            runs.get(run_id).cloned()
        };
        let tx = tx?;
        let rx = tx.subscribe();
        // Wrap the broadcast receiver in a Stream using unfold — avoids
        // BroadcastStream trait resolution issues.
        let stream = stream::unfold(rx, |mut rx| async move {
            loop {
                match rx.recv().await {
                    Ok(event) => return Some((event, rx)),
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(lagged = n, "AG-UI SSE subscriber lagged, skipping events");
                        // loop back to next recv
                    }
                    Err(broadcast::error::RecvError::Closed) => return None,
                }
            }
        });
        Some(stream)
    }

    /// Clean up a completed/errored run's channel.
    pub async fn cleanup_run(&self, run_id: &str) {
        let mut runs = self.inner.runs.lock().await;
        runs.remove(run_id);
    }
}

impl Default for AgUiState {
    fn default() -> Self {
        Self::new(256)
    }
}
