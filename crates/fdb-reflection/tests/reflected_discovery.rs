//! Cross-check schema-filtered MCP discovery and OpenAPI RPC argument descriptions.
use fdb_reflection::compilers::{mcp::McpCompiler, openapi::OpenApiCompiler};
use fdb_reflection::{ArgMeta, DatabaseModel, FnMeta, ViewMeta};
use serde_json::json;

#[test]
fn schema_selection_applies_to_views_and_function_signatures() {
    let mut model = DatabaseModel {
        version: 8,
        tables: vec![],
        functions: vec![],
        views: vec![],
    };
    let args = [
        ("price", "numeric"),
        ("available_on", "date"),
        ("refreshed_at", "timestamptz"),
        ("metadata", "jsonb"),
        ("quantity", "smallint"),
        ("published", "boolean"),
        ("tag", "inventory_tag"),
    ];
    for schema in ["inventory", "archive", "flint_meta"] {
        model.views.push(ViewMeta {
            schema: schema.into(),
            name: "in-stock".into(),
            columns: vec![],
            security_barrier: true,
        });
        model.functions.push(FnMeta {
            schema: schema.into(),
            name: "find".into(),
            return_type: "record".into(),
            security_definer: false,
            args: args
                .iter()
                .map(|(name, pg_type)| ArgMeta {
                    name: (*name).into(),
                    pg_type: (*pg_type).into(),
                })
                .collect(),
        });
    }

    let all = McpCompiler::compile(&model);
    assert_eq!(McpCompiler::compile_filtered(&model, None), all);
    assert_eq!(all["tools"].as_array().expect("tools").len(), 4);
    let selected =
        McpCompiler::compile_filtered(&model, Some(&["inventory".into(), "flint_meta".into()]));
    let tools = selected["tools"].as_array().expect("tools");
    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0]["name"], "list_inventory_in_stock");
    assert_eq!(tools[1]["name"], "call_inventory_find");
    assert_eq!(
        tools[1]["inputSchema"]["properties"]["price"]["type"],
        "number"
    );
    assert_eq!(
        tools[1]["inputSchema"]["properties"]["metadata"]["type"],
        "object"
    );

    let openapi = OpenApiCompiler::compile(&model);
    let props = &openapi["paths"]["/rpc/inventory/find"]["post"]["requestBody"]["content"]
        ["application/json"]["schema"]["properties"];
    assert_eq!(
        props,
        &json!({
            "price":{"type":"number"}, "available_on":{"type":"string","format":"date"},
            "refreshed_at":{"type":"string","format":"date-time"}, "metadata":{},
            "quantity":{"type":"integer"}, "published":{"type":"boolean"},
            "tag":{"type":"string","description":"Postgres type: inventory_tag"},
        })
    );
}
