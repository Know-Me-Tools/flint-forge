//! Flint Quarry port traits — the hexagonal seams. No adapter ever appears here.
#![forbid(unsafe_code)]

pub mod keto;

use async_trait::async_trait;
pub use keto::KetoCheck;

use fdb_domain::{
    ChangeEvent, GraphQlRequest, RestQuery, RestResult, SchemaVersion, SubscriptionSpec, TableMeta,
};
use forge_domain::Json;
use forge_identity::RlsContext;
use futures::stream::BoxStream;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BackendError {
    #[error("connection")]
    Connection,
    #[error("query: {0}")]
    Query(String),
    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StreamError {
    #[error("source unavailable")]
    Unavailable,
    #[error("authz denied")]
    Denied,
}

/// An acquired, RLS-scoped connection handle. The inner value is an opaque
/// adapter-specific type; adapters downcast it to the concrete connection.
pub struct Conn(pub Box<dyn std::any::Any + Send>);

#[async_trait]
pub trait DatabaseBackend: Send + Sync {
    /// Acquire a pooled connection with ROLE + request.jwt.claims + request.headers set.
    async fn acquire(&self, rls: &RlsContext) -> Result<Conn, BackendError>;
}

#[async_trait]
pub trait SchemaProvider: Send + Sync {
    async fn introspect(&self) -> Result<Vec<TableMeta>, BackendError>;
    fn subscribe_ddl(&self) -> tokio::sync::watch::Receiver<SchemaVersion>;
}

#[async_trait]
pub trait RestExecutor: Send + Sync {
    async fn execute(&self, q: RestQuery, rls: &RlsContext) -> Result<RestResult, BackendError>;
}

/// Executes an already-rendered, parameter-bound SQL statement under the
/// caller's RLS context.
///
/// This is one layer lower than [`RestExecutor`]: the caller (`fdb-reflection`)
/// has already translated a PostgREST-style request into `(sql, params)` via
/// the pure `fdb-query` planner — including resource-embedding joins,
/// mutations, and `/rpc` calls that `RestQuery` cannot express. The adapter's
/// only remaining job is to `acquire(rls)` (open the transaction, issue the
/// `SET LOCAL` GUCs) and bind + run the statement. No RLS logic lives above
/// this trait; every caller gets tenant isolation for free by construction.
#[async_trait]
pub trait SqlExecutor: Send + Sync {
    /// Run `sql` with `params` bound in `$n` order, inside an RLS-scoped
    /// transaction. Returns each row as a JSON object keyed by column name.
    async fn execute_raw(
        &self,
        sql: &str,
        params: Vec<fdb_query::QueryParam>,
        rls: &RlsContext,
    ) -> Result<Vec<serde_json::Map<String, serde_json::Value>>, BackendError>;
}

#[async_trait]
pub trait GraphQlExecutor: Send + Sync {
    /// The reversibility seam — pg_graphql passthrough today, a second dialect tomorrow.
    async fn execute(&self, req: GraphQlRequest, rls: &RlsContext) -> Result<Json, BackendError>;
}

#[async_trait]
pub trait ChangeStreamSource: Send + Sync {
    async fn watch(
        &self,
        spec: SubscriptionSpec,
        who: &RlsContext,
    ) -> Result<BoxStream<'static, Result<ChangeEvent, StreamError>>, StreamError>;
}
