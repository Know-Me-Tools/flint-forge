use crate::model::DatabaseModel;

/// Compiles a `DatabaseModel` into MCP tool descriptors.
/// Implementation lands in Phase 7.
pub struct McpCompiler;

impl McpCompiler {
    pub fn compile(_model: &DatabaseModel) -> serde_json::Value {
        todo!("Phase 7: McpCompiler")
    }
}
