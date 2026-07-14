use std::sync::Arc;

use fdb_domain::ChangeEvent;

use super::error::redact;
use super::payload::parse_payload;
use super::RECONNECT_BACKOFF;

/// The single background task: own the `PgListener`, decode each notification once,
/// and fan it out over the broadcast. Runs for the process lifetime.
pub(super) async fn listen_loop(
    mut listener: sqlx::postgres::PgListener,
    tx: tokio::sync::broadcast::Sender<Arc<ChangeEvent>>,
) {
    loop {
        match listener.recv().await {
            Ok(notif) => match parse_payload(notif.payload()) {
                Ok((_tenant, event)) => {
                    // Ignore SendError: zero receivers just means no active
                    // subscribers right now — that is normal, not an error.
                    let _ = tx.send(Arc::new(event));
                }
                Err(_) => {
                    // Malformed payload → count + drop; NEVER panic, NEVER break.
                    // Do NOT log the payload (it may carry row data / PII).
                    tracing::warn!("dropped unparseable change notification");
                }
            },
            Err(e) => {
                // Connection dropped. sqlx `PgListener` auto-reconnects and
                // re-issues LISTEN on the next recv(); log (redacted) and back off.
                tracing::warn!(error = %redact(&e), "listen connection error; reconnecting");
                tokio::time::sleep(RECONNECT_BACKOFF).await;
            }
        }
    }
}
