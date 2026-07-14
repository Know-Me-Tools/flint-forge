//! Row types for `flint_a2ui` queries.

use serde_json::Value;
use sqlx::{types::Json as SqlxJson, FromRow};

#[derive(Debug, FromRow)]
pub(super) struct RuleRow {
    pub(super) event_filter: SqlxJson<Value>,
    pub(super) assembly_config: SqlxJson<Value>,
    #[allow(dead_code)]
    pub(super) priority: i32,
}

#[derive(Debug, FromRow)]
pub(super) struct ComponentRow {
    #[allow(dead_code)]
    pub(super) slug: String,
    pub(super) primitive_type: String,
    #[allow(dead_code)]
    pub(super) schema: SqlxJson<Value>,
}

#[derive(Debug, FromRow)]
pub(super) struct BindingRow {
    #[allow(dead_code)]
    pub(super) table_schema: String,
    #[allow(dead_code)]
    pub(super) table_name: String,
    #[allow(dead_code)]
    pub(super) binding_type: String,
    pub(super) config: SqlxJson<Value>,
    #[allow(dead_code)]
    pub(super) slug: String,
    pub(super) primitive_type: String,
}
