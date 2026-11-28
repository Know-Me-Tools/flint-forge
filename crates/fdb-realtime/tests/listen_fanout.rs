//! Verify the fan-out, filtering and loss signal on the database's actual NOTIFY channel.
#![allow(clippy::expect_used)]

use fdb_domain::{ChangeOp, SubscriptionSpec};
use fdb_ports::{ChangeStreamSource, StreamError};
use fdb_realtime::{ListenChangeSource, ListenConfig};
use forge_identity::RlsContext;
use futures::StreamExt;
use serde_json::json;
use std::time::Duration;

#[tokio::test]
async fn independent_subscribers_receive_events_and_a_slow_reader_observes_loss() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        return;
    };
    let pool = sqlx::PgPool::connect(&url).await.expect("test database");
    let mut config = ListenConfig::new(url);
    config.broadcast_capacity = 2;
    let source = ListenChangeSource::new(config, None)
        .await
        .expect("listener");
    let who = RlsContext {
        role: "authenticated".into(),
        claims_json: "{}".into(),
        raw_bearer: String::new(),
        keto_subject: "subscriber".into(),
        vault_key_id: None,
    };
    let schema = format!("fanout_{}", std::process::id());
    let spec = SubscriptionSpec {
        entity_type: format!("{schema}.messages"),
        tenant: "tenant-a".into(),
        filter: None,
        checkpoint: None,
    };
    let mut invalid = spec.clone();
    invalid.checkpoint = Some("opaque".into());
    assert!(matches!(
        source.watch(invalid, &who).await,
        Err(StreamError::InvalidCheckpoint)
    ));
    let mut invalid = spec.clone();
    invalid.entity_type = String::new();
    assert!(matches!(
        source.watch(invalid, &who).await,
        Err(StreamError::Denied)
    ));
    let mut fast = source
        .watch(spec.clone(), &who)
        .await
        .expect("first reader");
    let mut slow = source.watch(spec, &who).await.expect("second reader");

    // Malformed payloads and events for a different table never reach the consumer.
    for payload in [
        "invalid".to_owned(),
        json!({"schema":schema,"table":"other","op":"insert"}).to_string(),
    ] {
        sqlx::query("SELECT pg_notify('flint_change',$1)")
            .bind(payload)
            .execute(&pool)
            .await
            .expect("notify");
    }
    for id in 1..=5 {
        let payload = json!({"schema":schema,"table":"messages","op":"update","record":{"id":id}});
        sqlx::query("SELECT pg_notify('flint_change',$1)")
            .bind(payload.to_string())
            .execute(&pool)
            .await
            .expect("notify");
        let event = tokio::time::timeout(Duration::from_secs(5), fast.next())
            .await
            .expect("event within deadline")
            .expect("open stream")
            .expect("fast reader kept up");
        assert_eq!(event.op, ChangeOp::Update);
        assert_eq!(event.record, Some(json!({"id":id})));
    }
    assert!(matches!(slow.next().await, Some(Err(StreamError::Lagged))));
}

#[tokio::test]
async fn invalid_listener_configuration_reports_a_redacted_connection_error() {
    let result = ListenChangeSource::new(ListenConfig::new("invalid scheme://"), None).await;
    assert!(matches!(result, Err(fdb_realtime::ListenError::Connect)));
}
