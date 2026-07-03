//! Map the reflection [`DatabaseModel`] into an [`fdb_query::EmbedSchema`].
//!
//! `fdb-query` is pure and knows nothing about the reflection model; the REST
//! router owns the translation. For each table we contribute its columns (for
//! validation) and, for every foreign key, TWO directed [`FkEdge`]s so an embed
//! can traverse the relationship either way:
//!
//! * the FK owner embedding its referenced parent — a **to-one** edge, and
//! * the referenced table embedding its dependent children — a **to-many** edge.
//!
//! FK names are synthesized as `<from_table>_<from_col>_fkey` (stable, validated
//! identifiers) so `!fk` disambiguation has something deterministic to match.

use std::collections::BTreeMap;

use fdb_query::embed::{Cardinality, EmbedSchema, FkEdge, TableDesc};

use crate::model::DatabaseModel;

/// Build an [`EmbedSchema`] from the live database model.
#[must_use]
pub fn embed_schema_from_model(model: &DatabaseModel) -> EmbedSchema {
    // Accumulate each table's columns + both edge directions before inserting,
    // since EmbedSchema is build-once (with_table) and a to-many edge must be
    // attached to the REFERENCED table, which may be processed in any order.
    let mut descs: BTreeMap<String, TableDesc> = BTreeMap::new();
    for table in &model.tables {
        let mut desc = TableDesc::new();
        for col in &table.columns {
            desc = desc.with_column(col.name.clone());
        }
        descs.insert(table.name.clone(), desc);
    }

    for table in &model.tables {
        for fk in &table.fk {
            let fk_name = format!("{}_{}_fkey", table.name, fk.from_col);

            // to-one: the FK owner embeds the single referenced row.
            if let Some(owner) = descs.get_mut(&table.name) {
                owner.fks.push(FkEdge {
                    fk_name: fk_name.clone(),
                    from_table: table.name.clone(),
                    from_col: fk.from_col.clone(),
                    to_table: fk.to_table.clone(),
                    to_col: fk.to_col.clone(),
                    cardinality: Cardinality::ToOne,
                });
            }

            // to-many: the referenced table embeds its dependent children.
            if let Some(referenced) = descs.get_mut(&fk.to_table) {
                referenced.fks.push(FkEdge {
                    fk_name,
                    from_table: table.name.clone(),
                    from_col: fk.from_col.clone(),
                    to_table: fk.to_table.clone(),
                    to_col: fk.to_col.clone(),
                    cardinality: Cardinality::ToMany,
                });
            }
        }
    }

    let mut schema = EmbedSchema::new();
    for (name, desc) in descs {
        schema = schema.with_table(name, desc);
    }
    schema
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Column, ForeignKey, Table};

    fn col(name: &str) -> Column {
        Column {
            name: name.into(),
            pg_type: "text".into(),
            nullable: true,
            default: None,
        }
    }

    fn model() -> DatabaseModel {
        // orders.customer_id -> customers.id
        DatabaseModel {
            tables: vec![
                Table {
                    schema: "public".into(),
                    name: "customers".into(),
                    columns: vec![col("id"), col("name")],
                    pk: vec!["id".into()],
                    fk: vec![],
                    rls_enabled: true,
                    vault_key: None,
                },
                Table {
                    schema: "public".into(),
                    name: "orders".into(),
                    columns: vec![col("id"), col("customer_id")],
                    pk: vec!["id".into()],
                    fk: vec![ForeignKey {
                        from_col: "customer_id".into(),
                        to_schema: "public".into(),
                        to_table: "customers".into(),
                        to_col: "id".into(),
                    }],
                    rls_enabled: true,
                    vault_key: None,
                },
            ],
            functions: vec![],
            views: vec![],
            version: 1,
        }
    }

    #[test]
    fn maps_columns_for_every_table() {
        let schema = embed_schema_from_model(&model());
        assert!(schema.table("customers").is_some());
        assert!(schema.table("orders").is_some());
    }

    #[test]
    fn fk_yields_to_one_on_owner_and_to_many_on_referenced() {
        let schema = embed_schema_from_model(&model());
        // orders embeds customers (to-one)
        let orders = schema.table("orders").expect("orders");
        assert!(
            orders
                .fks
                .iter()
                .any(|e| e.to_table == "customers" && matches!(e.cardinality, Cardinality::ToOne)),
            "orders should have a to-one edge to customers"
        );
        // customers embeds orders (to-many)
        let customers = schema.table("customers").expect("customers");
        assert!(
            customers
                .fks
                .iter()
                .any(|e| e.from_table == "orders" && matches!(e.cardinality, Cardinality::ToMany)),
            "customers should have a to-many edge from orders"
        );
    }

    #[test]
    fn fk_name_is_deterministic_and_safe() {
        let schema = embed_schema_from_model(&model());
        let orders = schema.table("orders").expect("orders");
        let edge = orders
            .fks
            .iter()
            .find(|e| e.to_table == "customers")
            .expect("edge");
        assert_eq!(edge.fk_name, "orders_customer_id_fkey");
        assert!(forge_domain::is_safe_identifier(&edge.fk_name));
    }
}
