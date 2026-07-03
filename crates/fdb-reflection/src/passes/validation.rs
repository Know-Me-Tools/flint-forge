use crate::{error::ReflectionError, model::DatabaseModel};

/// Validate structural invariants in the `DatabaseModel`.
/// Returns `ReflectionError::Validation` on the first invariant violation.
pub fn run(model: &DatabaseModel) -> Result<(), ReflectionError> {
    let table_names: std::collections::HashSet<(&str, &str)> = model
        .tables
        .iter()
        .map(|t| (t.schema.as_str(), t.name.as_str()))
        .collect();

    for table in &model.tables {
        if table.columns.is_empty() {
            return Err(ReflectionError::Validation(format!(
                "table {}.{} has no columns",
                table.schema, table.name
            )));
        }

        for fk in &table.fk {
            if !table_names.contains(&(fk.to_schema.as_str(), fk.to_table.as_str())) {
                return Err(ReflectionError::Validation(format!(
                    "table {}.{} has FK to unknown target {}.{}",
                    table.schema, table.name, fk.to_schema, fk.to_table
                )));
            }
        }

        // Guard: column names used in ORDER BY / SELECT must not be SQL keywords
        // that could participate in injection via identifier interpolation.
        // This is the static allowlist check — runtime checks are in the REST compiler.
        for col in &table.columns {
            if is_dangerous_identifier(&col.name) {
                return Err(ReflectionError::Validation(format!(
                    "column name '{}' in {}.{} is a reserved SQL keyword",
                    col.name, table.schema, table.name
                )));
            }
        }
    }

    Ok(())
}

/// Reject the small set of column names that, if interpolated into SQL, could
/// change statement semantics regardless of quoting context. In practice these
/// names should never come from a well-designed schema, but we reject them
/// at reflection time to prevent silent vulnerabilities if they do appear.
fn is_dangerous_identifier(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        ";" | "--" | "/*" | "*/" | "drop" | "truncate" | "exec" | "execute"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Column, ForeignKey, Table};

    fn col(name: &str) -> Column {
        Column {
            name: name.to_string(),
            pg_type: "text".to_string(),
            nullable: true,
            default: None,
        }
    }

    fn table(schema: &str, name: &str, columns: Vec<Column>, fk: Vec<ForeignKey>) -> Table {
        Table {
            schema: schema.to_string(),
            name: name.to_string(),
            columns,
            pk: vec![],
            fk,
            rls_enabled: true,
            vault_key: None,
        }
    }

    #[test]
    fn rejects_empty_table() {
        let model = DatabaseModel {
            tables: vec![table("public", "empty", vec![], vec![])],
            functions: vec![],
            views: vec![],
            version: 1,
        };
        assert!(run(&model).is_err());
    }

    #[test]
    fn rejects_unknown_fk_target() {
        let fk = ForeignKey {
            from_col: "other_id".to_string(),
            to_schema: "public".to_string(),
            to_table: "nonexistent".to_string(),
            to_col: "id".to_string(),
        };
        let model = DatabaseModel {
            tables: vec![table("public", "items", vec![col("id")], vec![fk])],
            functions: vec![],
            views: vec![],
            version: 1,
        };
        assert!(run(&model).is_err());
    }

    #[test]
    fn accepts_valid_model() {
        let model = DatabaseModel {
            tables: vec![table(
                "public",
                "items",
                vec![col("id"), col("name")],
                vec![],
            )],
            functions: vec![],
            views: vec![],
            version: 1,
        };
        assert!(run(&model).is_ok());
    }
}
