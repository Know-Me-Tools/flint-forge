//! Inner connection type held inside `fdb_ports::Conn`.
//!
//! This module lives entirely inside the adapter. `fdb-ports` knows only
//! the opaque `Conn(Box<dyn Any + Send>)` type; callers within `fdb-postgres`
//! downcast back via `PgConn::from_conn`.

use deadpool_postgres::Object;
use fdb_ports::BackendError;
use std::sync::atomic::{AtomicBool, Ordering};

/// The concrete connection held inside `fdb_ports::Conn`.
pub struct PgConn {
    inner: Option<Object>,
    finished: AtomicBool,
}

impl PgConn {
    /// Wrap a checked-out deadpool object.
    pub fn new(object: Object) -> Self {
        Self {
            inner: Some(object),
            finished: AtomicBool::new(false),
        }
    }

    pub(crate) fn client(&self) -> Result<&Object, BackendError> {
        self.inner.as_ref().ok_or(BackendError::Connection)
    }

    /// Downcast an opaque `fdb_ports::Conn` back to `PgConn`.
    /// Returns `None` if the inner value was not created by this adapter.
    pub fn from_conn(conn: &fdb_ports::Conn) -> Option<&Self> {
        conn.0.downcast_ref::<PgConn>()
    }

    /// Commit the RLS transaction opened by `PgBackend::acquire`.
    ///
    /// **Every caller that runs a statement with side effects MUST call this.**
    /// `acquire` issues `BEGIN` because its local role and identity GUCs only
    /// take effect inside a transaction. Without a successful `COMMIT`, this
    /// guard discards the connection, closing it and rolling back its work.
    /// Deadpool's default Fast recycling does not roll back transactions.
    /// An uncommitted statement's own
    /// `RETURNING` clause still reports success, because it reads inside the
    /// doomed transaction.
    ///
    /// Reads need not commit (a rolled-back `SELECT` returns correct rows), but
    /// committing is harmless and keeps the lifecycle uniform.
    ///
    /// This is deliberately explicit rather than a `Drop` guard: `Drop` cannot
    /// be async, so a drop-time commit would have to block the runtime or spawn
    /// a detached task that can fail silently — neither is acceptable on the
    /// path that enforces RLS.
    ///
    /// # Errors
    ///
    /// Returns [`fdb_ports::BackendError::Query`] if the `COMMIT` itself fails
    /// (for example, the transaction was already aborted by a prior error).
    pub async fn commit(&self) -> Result<(), fdb_ports::BackendError> {
        self.client()?
            .execute("COMMIT", &[])
            .await
            .map_err(|e| {
                fdb_ports::BackendError::Query(format!("commit: {}", crate::error::describe_pg(&e)))
            })
            .map(|_| self.finished.store(true, Ordering::Release))
    }

    pub(crate) async fn rollback(&self) -> Result<(), BackendError> {
        self.client()?
            .batch_execute("ROLLBACK")
            .await
            .map_err(|_| BackendError::Connection)?;
        self.finished.store(true, Ordering::Release);
        Ok(())
    }
}

impl Drop for PgConn {
    fn drop(&mut self) {
        if self.finished.load(Ordering::Acquire) {
            return;
        }
        if let Some(object) = self.inner.take() {
            // Detach before dropping: Fast recycling would retain an open
            // transaction, its locks and local identity. ClientWrapper's
            // Drop closes the connection; no async cleanup task is spawned.
            drop(Object::take(object));
        }
    }
}

#[cfg(test)]
#[path = "conn_tests.rs"]
mod tests;
