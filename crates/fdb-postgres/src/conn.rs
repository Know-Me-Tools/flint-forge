//! Inner connection type held inside `fdb_ports::Conn`.
//!
//! This module lives entirely inside the adapter. `fdb-ports` knows only
//! the opaque `Conn(Box<dyn Any + Send>)` type; callers within `fdb-postgres`
//! downcast back via `PgConn::from_conn`.

use deadpool_postgres::Object;

/// The concrete connection held inside `fdb_ports::Conn`.
pub struct PgConn {
    #[allow(dead_code)]
    pub(crate) inner: Object,
}

impl PgConn {
    /// Wrap a checked-out deadpool object.
    pub fn new(object: Object) -> Self {
        Self { inner: object }
    }

    /// Downcast an opaque `fdb_ports::Conn` back to `PgConn`.
    /// Returns `None` if the inner value was not created by this adapter.
    pub fn from_conn(conn: &fdb_ports::Conn) -> Option<&Self> {
        conn.0.downcast_ref::<PgConn>()
    }

    /// Commit the RLS transaction opened by `PgBackend::acquire`.
    ///
    /// **Every caller that runs a statement with side effects MUST call this.**
    /// `acquire` issues `BEGIN` because `SET LOCAL ROLE` and the five
    /// `set_config(..., is_local => true)` GUCs only take effect inside a
    /// transaction. That transaction stays open for the connection's whole
    /// lifetime; without a `COMMIT`, deadpool rolls it back when the object is
    /// recycled and **every write is silently discarded** — the statement's own
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
        self.inner
            .execute("COMMIT", &[])
            .await
            .map_err(|e| {
                fdb_ports::BackendError::Query(format!("commit: {}", crate::error::describe_pg(&e)))
            })
            .map(|_| ())
    }
}
