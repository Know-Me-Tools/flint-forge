use super::*;
use sqlx::postgres::PgPoolOptions;

use crate::gateway_fixture_database as fixture;

#[tokio::test]
async fn hook_events_reach_the_named_run_in_order_with_a_shared_call_id() {
    let agui = AgUiState::new(8);
    let mut target = agui.channel_for("run-a").await.subscribe();
    let mut other = agui.channel_for("run-b").await.subscribe();
    let payload = serde_json::json!({"schema":"inventory", "table":"widgets", "id":12});
    let row = OutboxRow {
        id: 1,
        payload: sqlx::types::Json(payload.clone()),
        agui_run_id: Some("run-a".to_owned()),
        retry_count: 0,
    };
    deliver_agui_event(&agui, &row).await.unwrap();
    let AgUiEvent::ToolCallStart {
        tool_call_id: started_id,
        tool_name,
        ..
    } = target.try_recv().expect("tool start")
    else {
        panic!("expected ToolCallStart")
    };
    assert_eq!(tool_name, "hook:inventory.widgets");
    Uuid::parse_str(&started_id).expect("unique tool call identifier");
    let AgUiEvent::ToolCallResult {
        tool_call_id,
        result,
        error,
    } = target.try_recv().expect("tool result")
    else {
        panic!("expected ToolCallResult")
    };
    assert_eq!(tool_call_id, started_id);
    assert_eq!(result, payload);
    assert!(error.is_none());
    assert!(matches!(
        other.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    ));
}

#[tokio::test]
async fn undeliverable_hooks_back_off_then_stop_retrying() {
    let Ok(admin_url) = std::env::var("DATABASE_URL") else {
        return;
    };
    fixture::with_database(&admin_url, |options| async move {
        let pool = PgPoolOptions::new().connect_with(options).await.unwrap();
        sqlx::query("CREATE EXTENSION \"ext-flint-hooks\" CASCADE")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO flint.webhook_outbox (webhook_id, payload, headers, target_type, retry_count)
             SELECT gen_random_uuid(), '{}'::jsonb, '{}'::jsonb, 'agui_run', n
             FROM generate_series(0, 4) AS n",
        )
        .execute(&pool)
        .await
        .unwrap();
        let agui = AgUiState::new(4);
        process_batch(&pool, &agui).await.unwrap();
        let attempts: Vec<(String, i32, i32)> = sqlx::query_as(
            "SELECT status, retry_count,
              round(EXTRACT(EPOCH FROM (visible_at - updated_at)))::int
             FROM flint.webhook_outbox ORDER BY retry_count",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(attempts.len(), 5);
        let schedule = [(1, 30), (2, 60), (3, 120), (4, 300)];
        for ((status, count, delay), expected) in attempts[..4].iter().zip(schedule) {
            assert_eq!(status, "retrying");
            assert_eq!((*count, *delay), expected);
        }
        assert_eq!(attempts[4].0, "failed");
        assert_eq!(attempts[4].1, 5);
        process_batch(&pool, &agui).await.unwrap();
        let sum: i64 = sqlx::query_scalar("SELECT SUM(retry_count) FROM flint.webhook_outbox")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(sum, 15);
        pool.close().await;
    }).await;
}
