//! Flint Quarry domain types.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use forge_domain::Json;
use serde::{Deserialize, Serialize};

/// Introspected shape of a single Postgres table or view, as returned by
/// `SchemaProvider::introspect`.
///
/// This is the schema-cache unit the REST/GraphQL compilers (`fdb-reflection`)
/// and the subscription pipeline (`fdb-app::build_pk_filters`) key off: it
/// tells them which columns exist, which columns form the primary key (needed
/// to re-query a changed row by identity), and whether the table is safe to
/// expose at all.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableMeta {
    /// Postgres schema the table lives in (e.g. `public`).
    pub schema: String,
    /// Table (or view) name, unqualified.
    pub name: String,
    /// Every column on the table, in database column order.
    pub columns: Vec<ColumnMeta>,
    /// Names of the columns that make up the primary key, in key order.
    ///
    /// Used to build per-row equality filters (see
    /// `fdb-app::build_pk_filters`) when re-querying a changed row for a
    /// subscription's RLS visibility check.
    pub primary_key: Vec<String>,
    /// Whether row-level security is enabled on the table.
    ///
    /// Tables without RLS enabled are excluded from generated REST/GraphQL/MCP
    /// surfaces (see `fdb-reflection`'s permission-analysis and GraphQL
    /// compiler passes) because there is no authoritative row filter to
    /// enforce for them.
    pub rls_enabled: bool,
}

/// Introspected shape of a single column on a [`TableMeta`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMeta {
    /// Column name.
    pub name: String,
    /// The column's Postgres type name (e.g. `text`, `int4`, `vector`).
    pub sql_type: String,
    /// Whether the column allows `NULL` values.
    pub nullable: bool,
}

/// The kind of write that produced a [`ChangeEvent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ChangeOp {
    /// A new row was inserted.
    Insert,
    /// An existing row was updated.
    Update,
    /// An existing row was deleted.
    Delete,
    /// An insert-or-update (`ON CONFLICT DO UPDATE`) was applied.
    Upsert,
}

/// A single row-level change delivered by a [`ChangeStreamSource`](fdb_ports).
///
/// `record`/`old_record` carry the row's data as JSON (post-image and
/// pre-image respectively); which is populated depends on `op` — inserts and
/// updates carry `record`, deletes carry `old_record`. Because WAL-sourced
/// change streams bypass Postgres RLS, every event must be re-validated
/// against the subscriber's RLS context before delivery (see
/// `Quarry::subscribe_rls_filtered`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeEvent {
    /// The kind of write that produced this event.
    pub op: ChangeOp,
    /// Schema of the table the change occurred on.
    pub schema: String,
    /// Table the change occurred on.
    pub table: String,
    /// The row's data after the change (present for inserts/updates).
    pub record: Option<Json>,
    /// The row's data before the change (present for updates/deletes).
    pub old_record: Option<Json>,
}

/// A parsed PostgREST-style REST query, ready for planning into SQL by
/// `fdb_query`.
///
/// Constructed from HTTP query-string parameters by the gateway's REST
/// routes. `select`/`order` use `fdb_query`'s PostgREST-compatible syntax
/// (parsed by `fdb_query::Select`/`fdb_query::Order`); `filters` are
/// already-split `(column, operator, value)` triples where `operator` is a
/// PostgREST operator token (e.g. `"eq"`, `"gt"`) resolved via
/// `fdb_query::Operator::parse`.
#[derive(Debug, Clone)]
pub struct RestQuery {
    /// Schema of the target table.
    pub schema: String,
    /// Name of the target table.
    pub table: String,
    /// PostgREST-style column/embed selection list (`select=col1,col2`), or
    /// `None` to select every column.
    pub select: Option<String>,
    /// Parsed filter conditions as `(column, operator, value)` triples,
    /// combined with logical AND.
    pub filters: Vec<(String, String, String)>,
    /// PostgREST-style order clause (`order=col.desc`), or `None` for
    /// unspecified order.
    pub order: Option<String>,
    /// Maximum number of rows to return, or `None` for no limit.
    pub limit: Option<u32>,
    /// Number of rows to skip before returning results, or `None` for no
    /// offset.
    pub offset: Option<u32>,
}

/// The result of executing a [`RestQuery`].
#[derive(Debug, Clone)]
pub struct RestResult {
    /// The matched rows, serialized as a JSON array of objects.
    pub rows: Json,
    /// The total number of matching rows, when a count was requested;
    /// `None` when counting was not performed.
    pub count: Option<u64>,
}

/// A GraphQL query/mutation request to be delegated to `graphql.resolve()`
/// inside Postgres under RLS.
#[derive(Debug, Clone)]
pub struct GraphQlRequest {
    /// The GraphQL document source.
    pub query: String,
    /// Variables for the operation, as a JSON object.
    pub variables: Json,
    /// The named operation to execute, when `query` defines more than one.
    pub operation_name: Option<String>,
}

/// A request to open a change-stream subscription for a single entity type
/// (table), used by `ChangeStreamSource::watch`.
#[derive(Debug, Clone)]
pub struct SubscriptionSpec {
    /// Tenant the subscriber belongs to; used for coarse Keto-level and
    /// fabric-level filtering of the change stream.
    pub tenant: String,
    /// The entity type to watch, formatted as `<schema>.<table>`.
    pub entity_type: String,
    /// Optional JSON filter narrowing which changes are delivered
    /// (reserved for future predicate-pushdown; currently informational).
    pub filter: Option<Json>,
}

/// Monotonically increasing schema generation counter.
///
/// Bumped whenever a DDL change invalidates the cached [`TableMeta`] set, so
/// consumers watching `SchemaProvider::subscribe_ddl` know to re-introspect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaVersion(pub u64);

/// Request body for `POST /rpc/vector` — vector similarity search.
///
/// The `embedding` is matched against `column` in `table` using the pgvector
/// cosine distance operator (`<=>`). Results are returned ordered by ascending
/// distance (most similar first).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct VectorRpcRequest {
    /// Query embedding as a flat array of f32 values.
    pub embedding: Vec<f32>,
    /// Qualified table name: `"<schema>.<table>"` or just `"<table>"` (defaults to `public`).
    pub table: String,
    /// Column name holding the `vector` values.
    pub column: String,
    /// Maximum number of results to return (default 10, capped at 1000).
    #[serde(default = "default_vector_limit")]
    pub limit: u32,
    /// Optional JSON filter applied as an additional WHERE clause (future use).
    pub filter: Option<serde_json::Value>,
}

fn default_vector_limit() -> u32 {
    10
}

// ─── AG-UI Event Types ──────────────────────────────────────────────────────

/// AG-UI protocol event types (v0.1 spec).
///
/// These are the standard lifecycle, text, tool-call, and state events
/// streamed to agent frontends over SSE at `/agents/v1/<run-id>/events`.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[allow(clippy::large_enum_variant)]
pub enum AgUiEvent {
    /// Sent when a run starts.
    #[serde(rename = "RunStarted")]
    RunStarted {
        /// Identifier of the run that started.
        run_id: String,
        /// Identifier of the conversation thread the run belongs to, if any.
        #[serde(default)]
        thread_id: Option<String>,
    },
    /// Marks the beginning of a text message from the agent.
    #[serde(rename = "TextMessageStart")]
    TextMessageStart {
        /// Identifier of the message being started.
        message_id: String,
        /// Role of the message sender (e.g. `"assistant"`).
        role: String,
    },
    /// A chunk of text content within a message.
    #[serde(rename = "TextMessageContent")]
    TextMessageContent {
        /// Identifier of the message this chunk belongs to.
        message_id: String,
        /// The text chunk to append to the message.
        content: String,
    },
    /// Marks the end of a text message.
    #[serde(rename = "TextMessageEnd")]
    TextMessageEnd {
        /// Identifier of the message that ended.
        message_id: String,
    },
    /// Marks the beginning of a tool call.
    #[serde(rename = "ToolCallStart")]
    ToolCallStart {
        /// Identifier of the tool call being started.
        tool_call_id: String,
        /// Name of the tool being invoked.
        tool_name: String,
        /// Identifier of the assistant message this tool call is attached
        /// to, if any.
        parent_message_id: Option<String>,
    },
    /// Arguments being streamed for a tool call (incremental JSON delta).
    #[serde(rename = "ToolCallArgs")]
    ToolCallArgs {
        /// Identifier of the tool call these arguments belong to.
        tool_call_id: String,
        /// Incremental JSON-encoded argument chunk to append.
        args: String,
    },
    /// Marks the end of a tool call (args are complete).
    #[serde(rename = "ToolCallEnd")]
    ToolCallEnd {
        /// Identifier of the tool call whose arguments are now complete.
        tool_call_id: String,
    },
    /// Result of a tool call, sent back to the agent/frontend.
    #[serde(rename = "ToolCallResult")]
    ToolCallResult {
        /// Identifier of the tool call this result answers.
        tool_call_id: String,
        /// The tool's return value.
        result: serde_json::Value,
        /// Error message, when the tool call failed instead of returning a
        /// result.
        #[serde(default)]
        error: Option<String>,
    },
    /// Full snapshot of agent state (MCP tools, catalog version, etc.).
    #[serde(rename = "StateSnapshot")]
    StateSnapshot {
        /// Identifier of the run this snapshot belongs to.
        run_id: String,
        /// The complete current state.
        state: serde_json::Value,
    },
    /// Incremental state change (JSON Patch format).
    #[serde(rename = "StateDelta")]
    StateDelta {
        /// Identifier of the run this delta belongs to.
        run_id: String,
        /// JSON Patch (RFC 6902) operations to apply to the run's state.
        delta: Vec<serde_json::Value>,
    },
    /// Custom event — used for A2UI surface delivery (`"a2ui:surface"`).
    #[serde(rename = "Custom")]
    Custom {
        /// Identifier of the run this custom event belongs to.
        run_id: String,
        /// Name identifying the kind of custom event (e.g. `"a2ui:surface"`).
        name: String,
        /// Event-specific payload.
        value: serde_json::Value,
    },
    /// Sent when a run completes successfully.
    #[serde(rename = "RunFinished")]
    RunFinished {
        /// Identifier of the run that finished.
        run_id: String,
    },
    /// Sent when a run fails.
    #[serde(rename = "RunError")]
    RunError {
        /// Identifier of the run that failed.
        run_id: String,
        /// Human-readable error message describing the failure.
        message: String,
    },
}

impl AgUiEvent {
    /// The run_id this event belongs to, if applicable.
    pub fn run_id(&self) -> Option<&str> {
        match self {
            AgUiEvent::RunStarted { run_id, .. }
            | AgUiEvent::StateSnapshot { run_id, .. }
            | AgUiEvent::StateDelta { run_id, .. }
            | AgUiEvent::Custom { run_id, .. }
            | AgUiEvent::RunFinished { run_id, .. }
            | AgUiEvent::RunError { run_id, .. } => Some(run_id.as_str()),
            _ => None,
        }
    }

    /// True if this event terminates the SSE stream for a run.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            AgUiEvent::RunFinished { .. } | AgUiEvent::RunError { .. }
        )
    }
}
