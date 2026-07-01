use crate::model::DatabaseModel;

/// Normalize the `DatabaseModel` in place:
/// - Deduplicate column names within each table (keep first occurrence)
/// - Lowercase all schema names
/// - Canonicalize common Postgres type aliases
pub fn run(model: &mut DatabaseModel) {
    for table in &mut model.tables {
        table.schema = table.schema.to_lowercase();

        // Deduplicate columns by name (keep first occurrence)
        let mut seen = std::collections::HashSet::new();
        table.columns.retain(|col| seen.insert(col.name.clone()));

        // Canonicalize type aliases
        for col in &mut table.columns {
            col.pg_type = canonicalize_pg_type(&col.pg_type);
        }
    }

    for func in &mut model.functions {
        func.schema = func.schema.to_lowercase();
    }

    for view in &mut model.views {
        view.schema = view.schema.to_lowercase();
    }
}

fn canonicalize_pg_type(pg_type: &str) -> String {
    match pg_type {
        "int4" => "integer",
        "int8" => "bigint",
        "int2" => "smallint",
        "float4" => "real",
        "float8" => "double precision",
        "bool" => "boolean",
        other => other,
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Column, Table};

    fn make_model(columns: Vec<Column>) -> DatabaseModel {
        DatabaseModel {
            tables: vec![Table {
                schema: "PUBLIC".to_string(),
                name: "items".to_string(),
                columns,
                pk: vec![],
                fk: vec![],
                rls_enabled: true,
                vault_key: None,
            }],
            functions: vec![],
            views: vec![],
            version: 1,
        }
    }

    #[test]
    fn lowercases_schema() {
        let mut model = make_model(vec![]);
        run(&mut model);
        assert_eq!(model.tables[0].schema, "public");
    }

    #[test]
    fn deduplicates_columns() {
        let col = |name: &str| Column {
            name: name.to_string(),
            pg_type: "text".to_string(),
            nullable: true,
            default: None,
        };
        let mut model = make_model(vec![col("id"), col("name"), col("id")]);
        run(&mut model);
        assert_eq!(model.tables[0].columns.len(), 2);
        assert_eq!(model.tables[0].columns[0].name, "id");
        assert_eq!(model.tables[0].columns[1].name, "name");
    }

    #[test]
    fn canonicalizes_int4_to_integer() {
        let mut model = make_model(vec![Column {
            name: "count".to_string(),
            pg_type: "int4".to_string(),
            nullable: false,
            default: None,
        }]);
        run(&mut model);
        assert_eq!(model.tables[0].columns[0].pg_type, "integer");
    }
}
