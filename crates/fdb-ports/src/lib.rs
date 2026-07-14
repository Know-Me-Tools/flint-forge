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

    /// Run a single SQL statement under the full RLS context and return each
    /// result row JSON-encoded (`{"column": value, ...}`).
    ///
    /// `params` are JSON-encoded scalar bind values, one per `$N` placeholder
    /// — the same "send raw text, let Postgres resolve the target type"
    /// binding strategy `RestExecutor` uses for untyped filter values, not a
    /// typed bind. `sql` may be a plain `SELECT` or a DML statement with
    /// `RETURNING`; a DML statement with no `RETURNING` succeeds with zero
    /// rows.
    ///
    /// Consumed by Flint Kiln's `flint:host/db` and `flint:host/llm` host
    /// implementations to forward a WASM component's governed SQL call
    /// through the same connection/RLS-context machinery REST and GraphQL
    /// already use — see `fke-runtime`.
    async fn query_json(
        &self,
        rls: &RlsContext,
        sql: &str,
        params: &[String],
    ) -> Result<Vec<String>, BackendError>;
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
