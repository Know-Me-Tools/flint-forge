use super::{json, Client, Value};

pub(super) async fn mcp_tools(client: &Client, table: &str, id: &str, context: &Value) {
    for (name, args, field) in [
        (
            "a2ui_list_components",
            json!({"category":"layout","app_id":id}),
            "components",
        ),
        ("a2ui_get_component", json!({"slug":"button"}), "component"),
        (
            "a2ui_generate_form",
            json!({"schema":"public","table":table}),
            "fields",
        ),
        (
            "a2ui_generate_grid",
            json!({"schema":"public","table":table}),
            "columns",
        ),
        (
            "a2ui_semantic_search",
            json!({"query":"button","limit":2}),
            "results",
        ),
        (
            "a2ui_resolve_tokens",
            json!({"application_slug":table}),
            "tokens",
        ),
        (
            "a2ui_assemble_surface",
            json!({"event_type":"mount","event_context":context}),
            "messages",
        ),
    ] {
        let response = client
            .rpc(
                "/mcp/v1/a2ui",
                "tools/call",
                json!({"name":name,"arguments":args}),
            )
            .await;
        let encoded = response["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("{name}: {response}"));
        let value: Value = serde_json::from_str(encoded).expect("tool result");
        assert!(value.get(field).is_some(), "{name}: {value}");
    }
}

pub(super) async fn a2a_tasks(client: &Client, id: &str, context: &Value) {
    for (task, input, field) in [
        ("a2ui.component.discover", json!({}), "components"),
        (
            "a2ui.component.discover",
            json!({"query":"button","limit":2,"app_id":id}),
            "components",
        ),
        ("a2ui.search.semantic", json!({"query":"button"}), "results"),
        (
            "a2ui.component.assemble",
            json!({"event_type":"mount","event_payload":context}),
            "surface",
        ),
    ] {
        let response = client
            .rpc(
                "/a2a/v1",
                "tasks/send",
                json!({"task":{"name":task,"id":"fixture","input":input}}),
            )
            .await;
        assert_eq!(
            response["result"]["task"]["state"], "completed",
            "{response}"
        );
        assert!(response["result"]["task"]["output"].get(field).is_some());
    }
}

pub(super) async fn request_errors(client: &Client) {
    for (name, arguments, code) in [
        ("a2ui_list_components", json!({"app_id":"reader"}), -32602),
        ("a2ui_get_component", json!({}), -32602),
        (
            "a2ui_get_component",
            json!({"slug":"unknown-fixture-component"}),
            -32603,
        ),
        ("a2ui_semantic_search", json!({}), -32602),
        ("a2ui_generate_grid", json!({"schema":"public"}), -32602),
        (
            "a2ui_generate_form",
            json!({"schema":"public","table":"unbound_fixture"}),
            -32602,
        ),
        (
            "a2ui_resolve_tokens",
            json!({"application_slug":"unknown-fixture-app"}),
            -32602,
        ),
        ("a2ui_assemble_surface", json!({}), -32602),
    ] {
        let response = client
            .rpc(
                "/mcp/v1/a2ui",
                "tools/call",
                json!({"name":name, "arguments":arguments}),
            )
            .await;
        assert_eq!(response["error"]["code"], code, "{name}: {response}");
    }
    for (name, input, code) in [
        (
            "a2ui.component.discover",
            json!({"app_id":"reader"}),
            -32602,
        ),
        ("a2ui.component.assemble", json!({}), -32602),
        (
            "a2ui.component.assemble",
            json!({"event_type":"mount","event_payload":{"data_source":{"table":"unbound_fixture"}}}),
            -32603,
        ),
        ("a2ui.search.semantic", json!({}), -32602),
    ] {
        let response = client
            .rpc(
                "/a2a/v1",
                "tasks/send",
                json!({"task":{"name":name,"input":input}}),
            )
            .await;
        assert_eq!(response["error"]["code"], code, "{name}: {response}");
    }
}

pub(super) async fn receive_mcp_ping(client: &Client) {
    use http_body_util::BodyExt as _;
    use std::time::Duration;
    let response = client
        .request("GET", "/mcp/v1/a2ui/sse", &Value::Null, Some(&client.token))
        .await;
    assert_eq!(response.headers()["content-type"], "text/event-stream");
    let mut body = response.into_body();
    tokio::time::timeout(Duration::from_secs(22), async {
        while let Some(frame) = body.frame().await {
            let frame = frame.expect("MCP event");
            if let Some(bytes) = frame.data_ref() {
                let text = std::str::from_utf8(bytes).expect("event text");
                if text.contains("event: ping") {
                    assert!(text.contains("data: {}"));
                    return;
                }
            }
        }
        panic!("connection closed before first ping");
    })
    .await
    .expect("MCP heartbeat");
}
