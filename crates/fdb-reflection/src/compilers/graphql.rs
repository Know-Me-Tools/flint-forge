//! GraphQL subscription schema compiler.
//!
//! Produces an `async_graphql::dynamic::Schema` with one `<TableName>Changes`
//! subscription field per RLS-enabled table in the `DatabaseModel`. The schema is
//! stored in `CompiledState` and served by the `graphql-transport-ws` handler added
//! in p3-c004.
//!
//! Query and Mutation are delegated to `graphql.resolve()` (pg_graphql passthrough)
//! and are NOT represented here — only the Subscription root is built dynamically.

use async_graphql::dynamic::{
    Field, FieldFuture, Object, Schema, Subscription, SubscriptionField, SubscriptionFieldFuture,
    TypeRef,
};
use futures::stream;

use crate::model::DatabaseModel;

/// Errors that can occur while building the dynamic subscription schema.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GraphQlCompileError {
    #[error("schema build error: {0}")]
    Build(String),
}

/// Compiles the per-table subscription schema from the live `DatabaseModel`.
pub struct GraphQlCompiler;

impl GraphQlCompiler {
    /// Build an `async_graphql::dynamic::Schema` with one `<TableName>Changes`
    /// subscription field per RLS-enabled table.
    ///
    /// Tables without RLS are excluded: the subscription RLS re-query (p3-c002)
    /// requires RLS to be on before events are safe to yield.
    pub fn compile(model: &DatabaseModel) -> Result<Schema, GraphQlCompileError> {
        // Minimal Query root required by async-graphql even when unused here
        // (Query/Mutation are handled by pg_graphql passthrough in p3-c001).
        let query = Object::new("Query").field(Field::new(
            "_placeholder",
            TypeRef::named(TypeRef::BOOLEAN),
            |_| FieldFuture::new(async { Ok(None::<async_graphql::Value>) }),
        ));

        let mut subscription = Subscription::new("Subscription");
        let mut schema_builder = Schema::build("Query", None, Some("Subscription")).register(query);

        for table in &model.tables {
            if !table.rls_enabled {
                continue;
            }

            let event_type_name = table_type_name(&table.schema, &table.name);

            // Per-column scalar fields plus the synthetic `_op` operation marker.
            let mut event_obj = Object::new(&event_type_name);
            event_obj = event_obj.field(Field::new(
                "_op",
                TypeRef::named_nn(TypeRef::STRING),
                |_| FieldFuture::new(async { Ok(None::<async_graphql::Value>) }),
            ));
            for col in &table.columns {
                let col_name = col.name.clone();
                event_obj = event_obj.field(Field::new(
                    col_name,
                    TypeRef::named(TypeRef::STRING),
                    |_| FieldFuture::new(async { Ok(None::<async_graphql::Value>) }),
                ));
            }

            // Subscription field: `<tableName>Changes` — yields a stream of event objects.
            // The live stream body is injected by `FabricChangeSource` in p3-c002;
            // this stub returns an empty stream so the schema validates correctly.
            let field_name = subscription_field_name(&table.name);
            let sub_field = SubscriptionField::new(
                field_name,
                TypeRef::named_nn_list_nn(&event_type_name),
                |_ctx| {
                    SubscriptionFieldFuture::new(async {
                        let s = stream::empty::<async_graphql::Result<async_graphql::Value>>();
                        Ok(s)
                    })
                },
            )
            .description(format!(
                "Real-time changes for the `{}` table (RLS enforced per event).",
                table.name
            ));

            subscription = subscription.field(sub_field);
            schema_builder = schema_builder.register(event_obj);
        }

        schema_builder = schema_builder.register(subscription);
        schema_builder
            .finish()
            .map_err(|e| GraphQlCompileError::Build(e.0))
    }
}

/// Converts a Postgres table name to PascalCase.
///
/// `"user_profiles"` → `"UserProfiles"`, `"orders"` → `"Orders"`.
fn pascal_case(name: &str) -> String {
    name.split('_')
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

/// `"public"`, `"user_profiles"` → `"UserProfilesChanges"`.
/// Schema prefix is omitted when schema is `"public"`.
fn table_type_name(schema: &str, table: &str) -> String {
    if schema == "public" {
        format!("{}Changes", pascal_case(table))
    } else {
        format!("{}{}Changes", pascal_case(schema), pascal_case(table))
    }
}

/// `"user_profiles"` → `"userProfilesChanges"` (camelCase subscription field name).
fn subscription_field_name(table: &str) -> String {
    let pascal = pascal_case(table);
    let mut chars = pascal.chars();
    match chars.next() {
        None => "changesChanges".into(),
        Some(first) => {
            let lower_first: String = first.to_lowercase().collect();
            format!("{}{}Changes", lower_first, chars.as_str())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Column, DatabaseModel, Table};

    fn make_model(tables: Vec<Table>) -> DatabaseModel {
        DatabaseModel {
            tables,
            functions: vec![],
            views: vec![],
            version: 1,
        }
    }

    fn make_table(name: &str, rls_enabled: bool) -> Table {
        Table {
            schema: "public".into(),
            name: name.into(),
            columns: vec![
                Column {
                    name: "id".into(),
                    pg_type: "uuid".into(),
                    nullable: false,
                    default: None,
                },
                Column {
                    name: "created_at".into(),
                    pg_type: "timestamptz".into(),
                    nullable: false,
                    default: None,
                },
            ],
            pk: vec!["id".into()],
            fk: vec![],
            rls_enabled,
            vault_key: None,
        }
    }

    #[test]
    fn pascal_case_converts_snake_to_pascal() {
        assert_eq!(pascal_case("user_profiles"), "UserProfiles");
        assert_eq!(pascal_case("orders"), "Orders");
        assert_eq!(pascal_case("line_item_events"), "LineItemEvents");
    }

    #[test]
    fn table_type_name_public_schema() {
        assert_eq!(table_type_name("public", "orders"), "OrdersChanges");
        assert_eq!(
            table_type_name("public", "user_profiles"),
            "UserProfilesChanges"
        );
    }

    #[test]
    fn table_type_name_non_public_schema() {
        assert_eq!(table_type_name("auth", "users"), "AuthUsersChanges");
    }

    #[test]
    fn subscription_field_name_is_camel_case() {
        assert_eq!(subscription_field_name("orders"), "ordersChanges");
        assert_eq!(
            subscription_field_name("user_profiles"),
            "userProfilesChanges"
        );
    }

    #[test]
    fn compile_succeeds_with_rls_tables() {
        let model = make_model(vec![
            make_table("orders", true),
            make_table("products", true),
        ]);
        let result = GraphQlCompiler::compile(&model);
        assert!(result.is_ok(), "compile should succeed: {result:?}");
    }

    #[test]
    fn compile_skips_non_rls_tables() {
        let model = make_model(vec![
            make_table("orders", true),
            make_table("internal_log", false),
        ]);
        let schema = GraphQlCompiler::compile(&model).expect("compile");
        let sdl = schema.sdl();
        assert!(sdl.contains("ordersChanges"), "should have ordersChanges field");
        assert!(
            !sdl.contains("InternalLogChanges"),
            "should not have non-RLS table"
        );
    }

    #[test]
    fn compiled_schema_sdl_has_subscription_type() {
        let model = make_model(vec![make_table("messages", true)]);
        let schema = GraphQlCompiler::compile(&model).expect("compile");
        let sdl = schema.sdl();
        assert!(
            sdl.contains("type Subscription"),
            "SDL should contain Subscription type"
        );
        assert!(
            sdl.contains("MessagesChanges"),
            "SDL should contain MessagesChanges type"
        );
    }
}
