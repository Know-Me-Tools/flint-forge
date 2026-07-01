//! Flint Quarry application layer — use-cases composed against ports.
#![forbid(unsafe_code)]

pub mod a2ui;
pub mod graphql;

use fdb_ports::{ChangeStreamSource, GraphQlExecutor, RestExecutor};
use std::sync::Arc;

/// Wires the use-cases over whatever adapters the interface layer injects.
pub struct Quarry {
    pub rest: Arc<dyn RestExecutor>,
    pub graphql: Arc<dyn GraphQlExecutor>,
    pub changes: Arc<dyn ChangeStreamSource>,
}

impl Quarry {
    pub fn new(
        rest: Arc<dyn RestExecutor>,
        graphql: Arc<dyn GraphQlExecutor>,
        changes: Arc<dyn ChangeStreamSource>,
    ) -> Self {
        Self {
            rest,
            graphql,
            changes,
        }
    }
}
