use std::sync::Arc;

use axum::Router;

use crate::model::DatabaseModel;

/// An atomic snapshot of all database-driven routing state.
///
/// Stored behind `ArcSwap<CompiledState>` so old guards held by in-flight
/// requests keep the previous state alive until those requests complete,
/// while new requests immediately see the updated state.
///
/// SECURITY: `DatabaseModel.tables[*].vault_key` contains `EncryptedDek`
/// (ciphertext only). Plaintext DEK is never stored here.
pub struct CompiledState {
    pub version: u64,
    pub database_model: Arc<DatabaseModel>,
    /// Compiled Axum router built from `DatabaseModel` by `RestCompiler`.
    /// Wrapped in `Arc` because `Router` is not `Clone`.
    pub router: Arc<Router>,
    pub openapi_doc: serde_json::Value,
    /// MCP tool definitions compiled from `DatabaseModel` by `McpCompiler`.
    /// Served at `/mcp/v1/tools`. Hot-swapped on DDL changes.
    pub mcp_tools_doc: serde_json::Value,
    /// Dynamic async-graphql schema exposing per-table `<TableName>Changes`
    /// subscription fields. `None` until `GraphQlCompiler::compile()` succeeds.
    /// Used by `graphql-transport-ws` in p3-c004.
    pub subscription_schema: Option<async_graphql::dynamic::Schema>,
    /// A2UI component catalog loaded from `flint_a2ui.components`.
    /// Empty when the flint_a2ui schema has not been deployed yet (graceful degradation).
    pub a2ui_catalog: Arc<A2uiCatalog>,
}

/// The full set of A2UI component definitions available to this Flint instance.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct A2uiCatalog {
    /// URI identifying this catalog, e.g. "/a2ui/v1/catalog/flint-base/1.0"
    pub catalog_id: String,
    pub version: String,
    pub components: Vec<A2uiCatalogEntry>,
}

impl A2uiCatalog {
    /// An empty catalog used when the flint_a2ui schema is not yet deployed.
    pub fn empty() -> Self {
        Self {
            catalog_id: String::new(),
            version: "0.0.0".into(),
            components: vec![],
        }
    }
}

/// A single component entry in the A2UI catalog.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct A2uiCatalogEntry {
    pub slug: String,
    pub primitive_type: String,
    pub category: String,
    pub schema: serde_json::Value,
    pub description: Option<String>,
}

impl std::fmt::Debug for CompiledState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mcp_count = self
            .mcp_tools_doc
            .get("tools")
            .and_then(|t| t.as_array())
            .map(Vec::len)
            .unwrap_or(0);
        f.debug_struct("CompiledState")
            .field("version", &self.version)
            .field("tables", &self.database_model.tables.len())
            .field("a2ui_components", &self.a2ui_catalog.components.len())
            .field("mcp_tools", &mcp_count)
            .finish_non_exhaustive()
    }
}
