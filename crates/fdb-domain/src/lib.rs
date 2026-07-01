//! Flint Quarry domain types.
#![forbid(unsafe_code)]

use forge_domain::Json;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableMeta {
    pub schema: String,
    pub name: String,
    pub columns: Vec<ColumnMeta>,
    pub primary_key: Vec<String>,
    pub rls_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMeta {
    pub name: String,
    pub sql_type: String,
    pub nullable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ChangeOp {
    Insert,
    Update,
    Delete,
    Upsert,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeEvent {
    pub op: ChangeOp,
    pub schema: String,
    pub table: String,
    pub record: Option<Json>,
    pub old_record: Option<Json>,
}

#[derive(Debug, Clone)]
pub struct RestQuery {
    pub schema: String,
    pub table: String,
    pub select: Option<String>,
    pub filters: Vec<(String, String, String)>, // (column, op, value)
    pub order: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct RestResult {
    pub rows: Json,
    pub count: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct GraphQlRequest {
    pub query: String,
    pub variables: Json,
    pub operation_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SubscriptionSpec {
    pub tenant: String,
    pub entity_type: String,
    pub filter: Option<Json>,
}

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
