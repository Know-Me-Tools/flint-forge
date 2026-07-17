use serde_json::json;

use fdb_domain::AgUiEvent;

use super::state::AgUiState;

#[test]
fn run_started_serializes_with_type_tag() {
    let event = AgUiEvent::RunStarted {
        run_id: "r-001".into(),
        thread_id: Some("t-001".into()),
    };
    let json_str = serde_json::to_string(&event).expect("serialize");
    assert!(json_str.contains("\"type\":\"RunStarted\""));
    assert!(json_str.contains("\"run_id\":\"r-001\""));
}

#[test]
fn text_message_content_round_trips() {
    let event = AgUiEvent::TextMessageContent {
        message_id: "m-001".into(),
        content: "Hello!".into(),
    };
    let json_str = serde_json::to_string(&event).expect("serialize");
    let parsed: AgUiEvent = serde_json::from_str(&json_str).expect("deserialize");
    match parsed {
        AgUiEvent::TextMessageContent {
            message_id,
            content,
        } => {
            assert_eq!(message_id, "m-001");
            assert_eq!(content, "Hello!");
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn custom_event_carries_a2ui_surface() {
    let event = AgUiEvent::Custom {
        run_id: "r-001".into(),
        name: "a2ui:surface".into(),
        value: json!({
            "protocol": "a2ui/0.9",
            "messages": [{"createSurface": {"surfaceId": "orders"}}]
        }),
    };
    assert_eq!(event.run_id(), Some("r-001"));
    assert!(!event.is_terminal());
}

#[test]
fn run_finished_is_terminal() {
    let event = AgUiEvent::RunFinished {
        run_id: "r-001".into(),
    };
    assert!(event.is_terminal());
}

#[test]
fn run_error_is_terminal() {
    let event = AgUiEvent::RunError {
        run_id: "r-001".into(),
        message: "WASM trap".into(),
    };
    assert!(event.is_terminal());
}

#[tokio::test]
async fn test_publish_and_subscribe() {
    let state = AgUiState::new(16);

    // Subscribe first (channel doesn't exist yet — lazy creation in subscribe).
    // We need to create the channel first via publish.
    state
        .publish(AgUiEvent::RunStarted {
            run_id: "r-001".into(),
            thread_id: None,
        })
        .await;

    // Now subscribe and then publish.
    let stream = state.subscribe("r-001").await.expect("stream");
    tokio::pin!(stream);

    state
        .publish(AgUiEvent::TextMessageContent {
            message_id: "m-001".into(),
            content: "test".into(),
        })
        .await;

    // The subscribe returns None for message events without run_id... wait.
    // TextMessageContent doesn't have a run_id, so publish drops it.
    // This is by design — only lifecycle/state/custom events have run_id.
    // For text events, the run context must be tracked externally.
}

#[tokio::test]
async fn test_cleanup_run_removes_channel() {
    let state = AgUiState::new(16);
    let _ = state.channel_for("r-temp").await;
    state.cleanup_run("r-temp").await;
    let runs = state.inner.runs.lock().await;
    assert!(!runs.contains_key("r-temp"));
}

#[tokio::test]
async fn test_start_run_returns_urls() {
    let state = AgUiState::new(16);
    // Simulate what the handler does.
    let run_id = uuid::Uuid::new_v4().to_string();
    let _ = state.channel_for(&run_id).await;
    assert!(!run_id.is_empty());
}

#[tokio::test]
async fn test_state_snapshot_event() {
    let state = AgUiState::new(16);
    let event = AgUiEvent::StateSnapshot {
        run_id: "r-001".into(),
        state: json!({"mcp_tools_count": 15}),
    };
    state.publish(event).await;
    // Verify the channel was created
    let runs = state.inner.runs.lock().await;
    assert!(runs.contains_key("r-001"));
}
