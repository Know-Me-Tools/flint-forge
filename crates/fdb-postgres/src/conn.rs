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
}
