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
    /// Schema version this state was compiled from (`DatabaseModel::version`).
    /// Also broadcast on `StateManager`'s `watch::Sender<u64>` on every hot-swap.
    pub version: u64,
    /// The reflected database IR this state was compiled from.
    pub database_model: Arc<DatabaseModel>,
    /// Compiled Axum router built from `DatabaseModel` by `RestCompiler`.
    /// Wrapped in `Arc` because `Router` is not `Clone`.
    pub router: Arc<Router>,
    /// OpenAPI 3 document describing `router`'s REST surface, built by
    /// `OpenApiCompiler::compile` and served as-is (e.g. at `/openapi.json`).
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
    /// Semantic version of the catalog contents, e.g. `"1.0.0"`.
    pub version: String,
    /// The component definitions themselves.
    pub components: Vec<A2uiCatalogEntry>,
}

impl A2uiCatalog {
    /// An empty catalog used when the flint_a2ui schema is not yet deployed.
    #[must_use]
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
    /// Unique component identifier within the catalog, e.g. `"data-table"`.
    pub slug: String,
    /// The underlying A2UI primitive this component renders as (e.g. `"table"`,
    /// `"form"`, `"chart"`).
    pub primitive_type: String,
    /// Grouping used for catalog browsing/ordering (`flint_a2ui.components`
    /// is queried `ORDER BY category, slug`).
    pub category: String,
    /// JSON Schema describing this component's configurable properties.
    pub schema: serde_json::Value,
    /// Human-readable description of the component, if provided.
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
