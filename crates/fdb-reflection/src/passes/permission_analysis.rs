use crate::model::DatabaseModel;

/// Emit tracing warnings for tables exposed to `anon` without RLS.
/// Phase 2: warn only. Phase 4 Cedar policy check will block.
pub fn run(model: &DatabaseModel) {
    for table in &model.tables {
        if !table.rls_enabled {
            tracing::warn!(
                schema = %table.schema,
                table = %table.name,
                "table has no RLS — all rows are visible to anon role"
            );
        }
    }
}
