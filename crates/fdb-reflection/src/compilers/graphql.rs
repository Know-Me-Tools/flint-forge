//! GraphQL subscription schema compiler.
//!
//! Produces an `async_graphql::dynamic::Schema` with one `<TableName>Changes`
//! subscription field per RLS-enabled table in the `DatabaseModel`. The schema is
//! stored in `CompiledState` and served by the `graphql-transport-ws` handler added
//! in p3-c004.
//!
//! Query and Mutation are delegated to `graphql.resolve()` (pg_graphql passthrough)
//! and are NOT represented here — only the Subscription root is built dynamically.

use std::sync::Arc;

use async_graphql::dynamic::{
    Field, FieldFuture, Object, ResolverContext, Schema, Subscription, SubscriptionField,
    SubscriptionFieldFuture, TypeRef,
};
use fdb_domain::{SubscriptionSpec, TableMeta};
use forge_identity::RlsContext;
use futures::stream::{self, BoxStream, StreamExt};

use crate::model::{DatabaseModel, Table};

/// The live-stream seam for GraphQL subscriptions.
///
/// A subscription field, when a client subscribes, calls this factory with the
/// table's `SubscriptionSpec`, its `TableMeta`, and the subscriber's `RlsContext`.
/// The factory returns the RLS-filtered stream of change events already projected
/// to `async_graphql::Value` objects.
///
/// It is defined in ports/domain terms only (no `fdb-app` dependency): the concrete
/// closure is built in the composition root (`fdb-gateway`), which wraps
/// `Quarry::subscribe_rls_filtered`. This keeps the hexagonal layering intact —
/// `fdb-reflection` depends on `fdb-domain`/`forge-identity`, never on `fdb-app`.
///
/// SECURITY: the `RlsContext` passed here carries `keto_subject` (PII) — it MUST NOT
/// be logged. The factory is responsible for the per-event RLS re-query; this compiler
/// never yields an event that has not passed through the factory.
pub type SubStreamFactory = Arc<
    dyn Fn(
            SubscriptionSpec,
            TableMeta,
            RlsContext,
        ) -> BoxStream<'static, async_graphql::Result<async_graphql::Value>>
        + Send
        + Sync,
>;

/// Errors that can occur while building the dynamic subscription schema.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GraphQlCompileError {
    /// `async_graphql::dynamic::Schema::finish()` rejected the assembled
    /// schema (e.g. a type/name collision between reflected tables, such as
    /// two tables producing the same PascalCase `<Name>Changes` type).
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
    ///
    /// When `factory` is `Some`, each subscription field yields the live
    /// RLS-filtered stream produced by the factory. When `None` (e.g. early boot
    /// before the composition root has wired the `Quarry`), fields yield an empty
    /// stream so the schema still validates. This never fails open: absence of a
    /// factory means no events, not unfiltered events.
    ///
    /// # Errors
    ///
    /// Returns [`GraphQlCompileError::Build`] if `async_graphql` rejects the
    /// assembled schema, e.g. two tables whose PascalCase `<Name>Changes`
    /// type names collide.
    pub fn compile(
        model: &DatabaseModel,
        factory: Option<SubStreamFactory>,
    ) -> Result<Schema, GraphQlCompileError> {
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
            // On subscribe, the field pulls the subscriber's `RlsContext` from the
            // connection data (installed by the gateway's `on_connection_init`) and
            // calls the injected `factory`, which returns the RLS-filtered event stream.
            let field_name = subscription_field_name(&table.name);
            let spec = table_subscription_spec(table);
            let table_meta = table_to_meta(table);
            let field_factory = factory.clone();
            let sub_field = SubscriptionField::new(
                field_name,
                TypeRef::named_nn_list_nn(&event_type_name),
                move |ctx: ResolverContext| {
                    let spec = spec.clone();
                    let table_meta = table_meta.clone();
                    let field_factory = field_factory.clone();
                    // Resolve the subscriber's RlsContext synchronously from connection
                    // data BEFORE entering the async block, so a missing context fails
                    // closed with an error stream rather than an unfiltered one.
                    let who = ctx.data::<RlsContext>().ok().cloned();
                    SubscriptionFieldFuture::new(async move {
                        match (field_factory, who) {
                            (Some(factory), Some(who)) => Ok(factory(spec, table_meta, who)),
                            // Fail closed: no factory wired (early boot) or no RLS context
                            // on the connection → yield no events. Never yield unfiltered.
                            (None, _) => Ok(stream::empty::<
                                async_graphql::Result<async_graphql::Value>,
                            >()
                            .boxed()),
                            (Some(_), None) => {
                                let err = async_graphql::Error::new(
                                    "subscription requires an authenticated connection",
                                );
                                Ok(stream::once(async move { Err(err) }).boxed())
                            }
                        }
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

/// Qualified entity-type identifier for a table: `"<schema>.<name>"`.
///
/// This is what `ChangeStreamSource::watch` keys on (the FRF entity type) and what
/// the RLS re-query uses to reach the right table.
fn table_entity_type(table: &Table) -> String {
    format!("{}.{}", table.schema, table.name)
}

/// Build the `SubscriptionSpec` for a table. `tenant` is left empty here and is
/// filled in by the factory from the subscriber's `RlsContext` at subscribe time —
/// the compiler has no per-subscriber context.
fn table_subscription_spec(table: &Table) -> SubscriptionSpec {
    SubscriptionSpec {
        tenant: String::new(),
        entity_type: table_entity_type(table),
        filter: None,
    }
}

/// Convert the reflection-layer `Table` into the domain `TableMeta` the RLS
/// re-query needs (schema, name, columns, primary key).
fn table_to_meta(table: &Table) -> TableMeta {
    TableMeta {
        schema: table.schema.clone(),
        name: table.name.clone(),
        columns: table
            .columns
            .iter()
            .map(|c| fdb_domain::ColumnMeta {
                name: c.name.clone(),
                sql_type: c.pg_type.clone(),
                nullable: c.nullable,
            })
            .collect(),
        primary_key: table.pk.clone(),
        rls_enabled: table.rls_enabled,
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
        let result = GraphQlCompiler::compile(&model, None);
        assert!(result.is_ok(), "compile should succeed: {result:?}");
    }

    #[test]
    fn compile_skips_non_rls_tables() {
        let model = make_model(vec![
            make_table("orders", true),
            make_table("internal_log", false),
        ]);
        let schema = GraphQlCompiler::compile(&model, None).expect("compile");
        let sdl = schema.sdl();
        assert!(
            sdl.contains("ordersChanges"),
            "should have ordersChanges field"
        );
        assert!(
            !sdl.contains("InternalLogChanges"),
            "should not have non-RLS table"
        );
    }

    #[test]
    fn compiled_schema_sdl_has_subscription_type() {
        let model = make_model(vec![make_table("messages", true)]);
        let schema = GraphQlCompiler::compile(&model, None).expect("compile");
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
