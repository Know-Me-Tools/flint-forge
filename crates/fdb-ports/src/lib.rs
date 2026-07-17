//! Flint Quarry port traits — the hexagonal seams. No adapter ever appears here.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod keto;

use async_trait::async_trait;
pub use keto::KetoCheck;

use fdb_domain::{
    ChangeEvent, GraphQlRequest, RestQuery, RestResult, SchemaVersion, SubscriptionSpec, TableMeta,
};
use forge_domain::Json;
use forge_identity::RlsContext;
use futures::stream::BoxStream;

/// Error surfaced by the [`DatabaseBackend`], [`SchemaProvider`],
/// [`RestExecutor`], [`SqlExecutor`], and [`GraphQlExecutor`] ports.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BackendError {
    /// A pooled connection could not be checked out, or the RLS-context
    /// transaction setup (`BEGIN` / `SET LOCAL` / `set_config`) failed.
    #[error("connection")]
    Connection,
    /// The request could not be turned into a valid query, or Postgres
    /// rejected the rendered SQL (syntax error, unknown operator, unsafe
    /// identifier, etc.).
    #[error("query: {0}")]
    Query(String),
    /// An adapter-internal failure not covered by the other variants (e.g. a
    /// mismatched connection type, or a downstream call such as
    /// `graphql.resolve()` erroring).
    #[error("internal: {0}")]
    Internal(String),
}

/// Error surfaced by the [`ChangeStreamSource`] port.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StreamError {
    /// The change-stream source (fabric gRPC, LISTEN/NOTIFY bridge) could not
    /// be reached or errored while opening the stream.
    #[error("source unavailable")]
    Unavailable,
    /// The subscriber failed the coarse Keto relationship check for the
    /// requested entity type (fail-closed: also returned when Keto itself is
    /// unreachable).
    #[error("authz denied")]
    Denied,
}

/// An acquired, RLS-scoped connection handle. The inner value is an opaque
/// adapter-specific type; adapters downcast it to the concrete connection.
pub struct Conn(
    /// Adapter-specific connection value (e.g. `fdb-postgres`'s `PgConn`),
    /// type-erased so this crate never depends on a concrete adapter.
    pub Box<dyn std::any::Any + Send>,
);

/// Acquires RLS-scoped connections from the underlying database.
#[async_trait]
pub trait DatabaseBackend: Send + Sync {
    /// Acquire a pooled connection with `ROLE`, `request.jwt.claims`, and
    /// `request.headers` set for the duration of the returned connection's
    /// transaction, so every subsequent statement runs under the caller's
    /// Postgres RLS context.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Connection`] when no pooled connection is
    /// available or the `SET LOCAL`/`set_config` GUC setup fails, and
    /// [`BackendError::Query`] when `rls.role` fails the safe-identifier
    /// check (fail-closed rather than interpolating an unvalidated role into
    /// `SET LOCAL ROLE`).
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

/// Introspects the database schema and reports on schema-changing DDL.
#[async_trait]
pub trait SchemaProvider: Send + Sync {
    /// Introspect every user-facing table/view, returning its columns,
    /// primary key, and RLS status as [`TableMeta`].
    ///
    /// # Errors
    ///
    /// Returns [`BackendError`] when the introspection connection cannot be
    /// acquired or the underlying catalog query fails.
    async fn introspect(&self) -> Result<Vec<TableMeta>, BackendError>;

    /// A watch channel that ticks a new [`SchemaVersion`] whenever DDL
    /// invalidates the cached [`TableMeta`] set, so consumers know to
    /// re-introspect.
    fn subscribe_ddl(&self) -> tokio::sync::watch::Receiver<SchemaVersion>;
}

/// Executes PostgREST-style [`RestQuery`] read requests under RLS.
#[async_trait]
pub trait RestExecutor: Send + Sync {
    /// Execute `q` under `rls`'s row-level-security context, returning the
    /// matched rows.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Connection`] when the RLS-scoped connection
    /// cannot be acquired, and [`BackendError::Query`] when `q` cannot be
    /// planned/rendered into valid SQL or Postgres rejects the statement.
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
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Connection`] when the RLS-scoped connection
    /// cannot be acquired, and [`BackendError::Query`]/[`BackendError::Internal`]
    /// when Postgres rejects `sql`/`params` or execution otherwise fails.
    async fn execute_raw(
        &self,
        sql: &str,
        params: Vec<fdb_query::QueryParam>,
        rls: &RlsContext,
    ) -> Result<Vec<serde_json::Map<String, serde_json::Value>>, BackendError>;
}

/// Executes GraphQL query/mutation requests — the reversibility seam:
/// `pg_graphql` passthrough today, a second dialect tomorrow.
#[async_trait]
pub trait GraphQlExecutor: Send + Sync {
    /// Execute `req` under `rls`'s row-level-security context, returning the
    /// GraphQL response as JSON.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Connection`] when the RLS-scoped connection
    /// cannot be acquired, and [`BackendError::Internal`] when the GraphQL
    /// engine (e.g. `graphql.resolve()`) errors or its result cannot be
    /// parsed as JSON.
    async fn execute(&self, req: GraphQlRequest, rls: &RlsContext) -> Result<Json, BackendError>;
}

/// Opens per-table change streams for the subscription pipeline.
///
/// Implementations MUST perform a coarse Keto relationship check
/// (fail-closed) before opening the stream; the per-event RLS re-query that
/// makes delivery authoritative is layered on above this port (see
/// `Quarry::subscribe_rls_filtered`), not inside adapters.
#[async_trait]
pub trait ChangeStreamSource: Send + Sync {
    /// Open a change stream for `spec.entity_type`, scoped to `spec.tenant`,
    /// as subscriber `who`.
    ///
    /// # Errors
    ///
    /// Returns [`StreamError::Unavailable`] when the underlying source
    /// (fabric gRPC, LISTEN/NOTIFY bridge, or the Keto check itself) cannot
    /// be reached, and [`StreamError::Denied`] when the coarse Keto check
    /// finds the subscriber lacks the `view` relation on `spec.entity_type`.
    async fn watch(
        &self,
        spec: SubscriptionSpec,
        who: &RlsContext,
    ) -> Result<BoxStream<'static, Result<ChangeEvent, StreamError>>, StreamError>;
}
