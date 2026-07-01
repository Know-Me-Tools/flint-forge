//! GraphQL introspection merger.
//!
//! Merges a pg_graphql introspection result with the subscription SDL from
//! the dynamic async-graphql schema. pg_graphql provides Query/Mutation types;
//! async-graphql provides the Subscription type.
//!
//! This module is pure JSON — no IO, no database calls, no async.

use serde_json::Value;

/// Detect whether a GraphQL request body is an introspection query.
///
/// Checks for the presence of `__schema` or `__type` in the query string.
/// This is a heuristic — it is not a full parse — but is correct for the
/// standard introspection queries emitted by every major GraphQL client.
pub fn is_introspection_query(query: &str) -> bool {
    query.contains("__schema") || query.contains("__type")
}

/// Merges a pg_graphql introspection JSON result with subscription types from
/// the async-graphql dynamic schema SDL.
///
/// # Merge algorithm
///
/// 1. Extract `data.__schema.types[]` from `pg_result`.
/// 2. Parse the subscription SDL from the async-graphql schema and extract its type names.
/// 3. Append subscription types to the `types[]` array, deduplicating by name
///    (pg_graphql wins on collision — it owns Query and Mutation).
/// 4. Set `data.__schema.subscriptionType` to `{"name": "Subscription"}`.
///
/// If `pg_result` is not well-formed introspection JSON, it is returned unchanged.
pub struct IntrospectionMerger;

impl IntrospectionMerger {
    /// Merge pg_graphql introspection JSON with the subscription schema SDL.
    ///
    /// `pg_result` — the raw JSON from `graphql.resolve()` for an introspection query.
    /// `subscription_schema` — the compiled dynamic schema from `GraphQlCompiler`.
    pub fn merge(
        mut pg_result: Value,
        subscription_schema: &async_graphql::dynamic::Schema,
    ) -> Value {
        // Extract the introspection type list.
        let Some(schema_obj) = pg_result
            .get_mut("data")
            .and_then(|d| d.get_mut("__schema"))
            .and_then(|s| s.as_object_mut())
        else {
            // Not a standard introspection response — return unchanged.
            return pg_result;
        };

        // Parse subscription type names from the SDL.
        let sdl = subscription_schema.sdl();
        let sub_types = extract_subscription_types_from_sdl(&sdl);

        // Merge type entries into pg_result, deduplicating by name (pg_graphql wins).
        let types_arr = schema_obj
            .entry("types")
            .or_insert_with(|| Value::Array(vec![]))
            .as_array_mut();

        if let Some(types) = types_arr {
            // Collect existing type names so we can skip duplicates.
            let existing: std::collections::HashSet<String> = types
                .iter()
                .filter_map(|t| t.get("name").and_then(Value::as_str).map(ToOwned::to_owned))
                .collect();

            for type_name in sub_types {
                if !existing.contains(&type_name) {
                    types.push(serde_json::json!({
                        "kind": "OBJECT",
                        "name": type_name,
                        "description": null,
                        "fields": [],
                        "inputFields": null,
                        "interfaces": [],
                        "enumValues": null,
                        "possibleTypes": null
                    }));
                }
            }
        }

        // Set the subscriptionType pointer.
        schema_obj.insert(
            "subscriptionType".to_owned(),
            serde_json::json!({"name": "Subscription"}),
        );

        pg_result
    }
}

/// Extract non-built-in type names from an async-graphql SDL string.
///
/// Returns names of `type` definitions that don't start with `__` (introspection types)
/// and aren't the built-in scalar or root types (`Query`, `Boolean`, `String`, etc.).
fn extract_subscription_types_from_sdl(sdl: &str) -> Vec<String> {
    let builtins: std::collections::HashSet<&str> = [
        "Boolean", "String", "Int", "Float", "ID", "Query", "__Schema",
        "__Type", "__Field", "__InputValue", "__EnumValue", "__Directive",
        "__DirectiveLocation",
    ]
    .iter()
    .copied()
    .collect();

    sdl.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("type ") {
                let name = rest.split_whitespace().next().unwrap_or("");
                if !name.is_empty() && !name.starts_with("__") && !builtins.contains(name) {
                    return Some(name.to_owned());
                }
            }
            None
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_schema_introspection() {
        assert!(is_introspection_query("{ __schema { types { name } } }"));
        assert!(is_introspection_query("query IntrospectionQuery { __schema { queryType { name } } }"));
    }

    #[test]
    fn detects_type_introspection() {
        assert!(is_introspection_query("{ __type(name: \"User\") { name fields { name } } }"));
    }

    #[test]
    fn does_not_flag_regular_queries() {
        assert!(!is_introspection_query("{ users { id name } }"));
        assert!(!is_introspection_query("mutation CreateUser($input: UserInput!) { createUser(input: $input) { id } }"));
    }

    #[test]
    fn merge_sets_subscription_type() {
        // Build a minimal introspection response.
        let pg_result = serde_json::json!({
            "data": {
                "__schema": {
                    "queryType": {"name": "Query"},
                    "mutationType": {"name": "Mutation"},
                    "subscriptionType": null,
                    "types": [
                        {"kind": "OBJECT", "name": "Query", "fields": [], "interfaces": [], "inputFields": null, "enumValues": null, "possibleTypes": null, "description": null},
                        {"kind": "OBJECT", "name": "User", "fields": [], "interfaces": [], "inputFields": null, "enumValues": null, "possibleTypes": null, "description": null}
                    ]
                }
            }
        });

        // Build a minimal subscription schema with one subscription type.
        use async_graphql::dynamic::{
            Field, FieldFuture, Object, Schema, Subscription, SubscriptionField,
            SubscriptionFieldFuture, TypeRef,
        };
        use futures::stream;

        let query = Object::new("Query").field(Field::new(
            "_p",
            TypeRef::named(TypeRef::BOOLEAN),
            |_| FieldFuture::new(async { Ok(None::<async_graphql::Value>) }),
        ));
        let orders_obj = Object::new("OrdersChanges").field(Field::new(
            "id",
            TypeRef::named(TypeRef::STRING),
            |_| FieldFuture::new(async { Ok(None::<async_graphql::Value>) }),
        ));
        let subscription = Subscription::new("Subscription").field(SubscriptionField::new(
            "ordersChanges",
            TypeRef::named_nn_list_nn("OrdersChanges"),
            |_| {
                SubscriptionFieldFuture::new(async {
                    Ok(stream::empty::<async_graphql::Result<async_graphql::Value>>())
                })
            },
        ));
        let schema = Schema::build("Query", None, Some("Subscription"))
            .register(query)
            .register(orders_obj)
            .register(subscription)
            .finish()
            .expect("schema build");

        let merged = IntrospectionMerger::merge(pg_result, &schema);

        // subscriptionType should now point to Subscription.
        let sub_type = &merged["data"]["__schema"]["subscriptionType"];
        assert_eq!(sub_type["name"], "Subscription");

        // OrdersChanges type should be in the types array.
        let types = merged["data"]["__schema"]["types"].as_array().expect("types array");
        let type_names: Vec<&str> = types
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        assert!(type_names.contains(&"OrdersChanges"), "types should contain OrdersChanges");
        // Original types should still be present.
        assert!(type_names.contains(&"Query"), "types should contain Query");
        assert!(type_names.contains(&"User"), "types should contain User");
    }

    #[test]
    fn merge_deduplicates_by_name_pg_graphql_wins() {
        // If pg_graphql already has a type named "Subscription", it should not be overwritten.
        let pg_result = serde_json::json!({
            "data": {
                "__schema": {
                    "queryType": {"name": "Query"},
                    "types": [
                        {"kind": "OBJECT", "name": "Query", "fields": ["pg_field"], "interfaces": [], "inputFields": null, "enumValues": null, "possibleTypes": null, "description": null},
                        {"kind": "OBJECT", "name": "Subscription", "fields": ["pg_sub_field"], "interfaces": [], "inputFields": null, "enumValues": null, "possibleTypes": null, "description": null}
                    ]
                }
            }
        });

        let schema = {
            use async_graphql::dynamic::{
                Field, FieldFuture, Object, Schema, Subscription, SubscriptionField,
                SubscriptionFieldFuture, TypeRef,
            };
            use futures::stream;
            let q = Object::new("Query").field(Field::new(
                "_p", TypeRef::named(TypeRef::BOOLEAN),
                |_| FieldFuture::new(async { Ok(None::<async_graphql::Value>) }),
            ));
            let sub = Subscription::new("Subscription").field(SubscriptionField::new(
                "changes", TypeRef::named(TypeRef::STRING),
                |_| SubscriptionFieldFuture::new(async { Ok(stream::empty::<async_graphql::Result<async_graphql::Value>>()) }),
            ));
            Schema::build("Query", None, Some("Subscription"))
                .register(q).register(sub).finish().expect("schema")
        };

        let merged = IntrospectionMerger::merge(pg_result, &schema);
        let types = merged["data"]["__schema"]["types"].as_array().expect("types");
        // There should be exactly one Subscription entry (pg_graphql's).
        let sub_entries: Vec<_> = types.iter().filter(|t| t["name"] == "Subscription").collect();
        assert_eq!(sub_entries.len(), 1, "should have exactly one Subscription type");
        // pg_graphql's version should have won (it has "pg_sub_field").
        assert_eq!(sub_entries[0]["fields"], serde_json::json!(["pg_sub_field"]));
    }

    #[test]
    fn returns_unchanged_when_not_introspection_format() {
        let not_introspection = serde_json::json!({"data": {"users": []}});
        let schema = {
            use async_graphql::dynamic::{Field, FieldFuture, Object, Schema, TypeRef};
            let q = Object::new("Query").field(Field::new(
                "_p", TypeRef::named(TypeRef::BOOLEAN),
                |_| FieldFuture::new(async { Ok(None::<async_graphql::Value>) }),
            ));
            Schema::build("Query", None, None).register(q).finish().expect("schema")
        };

        let result = IntrospectionMerger::merge(not_introspection.clone(), &schema);
        assert_eq!(result, not_introspection);
    }
}
