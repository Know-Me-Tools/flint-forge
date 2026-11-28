//! Exercise live and resumed subscriptions against a loopback Fabric v2 gRPC peer.
#![allow(clippy::expect_used)]
#[path = "support/fabric.rs"]
mod fabric;

use fabric::{checkpoint, entity, field, mutation, spec, subscriber, Fixture};
use fdb_domain::ChangeOp;
use fdb_ports::{ChangeStreamSource, StreamError};
use fdb_realtime::{FabricChangeSource, FabricError, FrfConfig};
use frf_proto::fv2::{
    canonical_value::Kind, watch_entity_type_request::Start, watch_entity_type_response::Frame,
    ChangeOp as WireOp, WatchAccepted,
};
use futures::StreamExt;
use serde_json::json;
use tonic::{Code, Status};

#[tokio::test]
async fn live_projection_and_key_only_delete_resume_with_the_server_checkpoint() {
    let mut updated = mutation(WireOp::Update);
    let fields = &mut updated.record.as_mut().expect("update record").fields;
    fields.extend([
        field("active", Kind::BoolValue(true)),
        field("size", Kind::UnsignedInteger(9)),
        field("price", Kind::DecimalValue("9.90".into())),
        field("score", Kind::FloatValue(2.5)),
        field("blob", Kind::BytesValue(vec![255])),
        field("remark", Kind::NullValue(1)),
        field("detail", Kind::JsonValue(br#"{"ok":true}"#.to_vec())),
        field("seen", Kind::TimestampValue(Default::default())),
    ]);
    let frames = vec![
        Frame::Accepted(WatchAccepted {
            entity_type: Some(entity()),
            tenant_id: spec().tenant,
            ..Default::default()
        }),
        Frame::CheckpointAdvanced(Default::default()),
        Frame::Mutation(mutation(WireOp::Insert)),
        Frame::Mutation(updated),
        Frame::Mutation(mutation(WireOp::Upsert)),
        Frame::Mutation(mutation(WireOp::Delete)),
    ];
    let mut fixture = Fixture::new(frames.into_iter().map(Ok).collect(), None).await;
    let mut events = fixture
        .source
        .watch(spec(), &subscriber())
        .await
        .expect("live watch");
    let (auth, request) = fixture.received.recv().await.expect("live request");
    assert_eq!(auth, "Bearer subscriber-token");
    assert_eq!(request.entity_type, Some(entity()));
    assert_eq!(request.tenant_id, spec().tenant);
    assert!(matches!(request.start, Some(Start::Live(_))));
    let inserted = events.next().await.expect("insert").expect("valid insert");
    assert_eq!(inserted.op, ChangeOp::Insert);
    let updated = events.next().await.expect("update").expect("valid update");
    assert_eq!(updated.op, ChangeOp::Update);
    assert_eq!(
        updated.record,
        Some(
            json!({"id":4,"active":true,"size":9,"price":"9.90","score":2.5,
        "blob":"_w","remark":null,"detail":{"ok":true},"seen":"1970-01-01T00:00:00Z"})
        )
    );
    assert_eq!(
        updated.projection,
        Some(
            ["id", "active", "size", "price", "score", "blob", "remark", "detail", "seen"]
                .map(str::to_owned)
                .to_vec()
        )
    );
    let upserted = events.next().await.expect("upsert").expect("valid upsert");
    assert_eq!(upserted.op, ChangeOp::Upsert);
    let deleted = events.next().await.expect("delete").expect("valid delete");
    assert!(deleted.delete_authorized);
    assert_eq!(deleted.op, ChangeOp::Delete);
    assert_eq!(deleted.record, None);
    assert_eq!(deleted.old_record, Some(json!({"id":4})));
    assert_eq!(deleted.event_id.as_deref(), Some("event-7"));
    assert!(events.next().await.is_none());
    let mut resume = spec();
    resume.checkpoint = Some(deleted.checkpoint.expect("server checkpoint"));
    let _resumed = fixture
        .source
        .watch(resume, &subscriber())
        .await
        .expect("resumed watch");
    let (_, request) = fixture.received.recv().await.expect("resume request");
    assert_eq!(request.start, Some(Start::Resume(checkpoint())));
}

#[tokio::test]
async fn timestamps_retain_their_utc_precision_across_the_wire() {
    let examples = [
        (0, 0, "1970-01-01T00:00:00Z"),
        (0, 100_000_000, "1970-01-01T00:00:00.100Z"),
        (-1, 1_000, "1969-12-31T23:59:59.000001Z"),
        (1, 12, "1970-01-01T00:00:01.000000012Z"),
    ];
    let frames = examples
        .iter()
        .map(|(seconds, nanos, _)| {
            let mut timestamp = Kind::TimestampValue(Default::default());
            if let Kind::TimestampValue(raw) = &mut timestamp {
                raw.seconds = *seconds;
                raw.nanos = *nanos;
            }
            let mut change = mutation(WireOp::Insert);
            change
                .record
                .as_mut()
                .expect("record")
                .fields
                .push(field("timestamp", timestamp));
            Ok(Frame::Mutation(change))
        })
        .collect();
    let fixture = Fixture::new(frames, None).await;
    let mut events = fixture
        .source
        .watch(spec(), &subscriber())
        .await
        .expect("watch");
    for (_, _, expected) in examples {
        let row = events
            .next()
            .await
            .expect("event")
            .expect("valid timestamp")
            .record
            .expect("record");
        assert_eq!(row["timestamp"], expected);
    }
}

#[tokio::test]
async fn invalid_frames_fail_closed_and_terminal_controls_retain_their_meaning() {
    let mut foreign = mutation(WireOp::Insert);
    foreign.tenant_id = "tenant-b".into();
    let mut duplicate_key = mutation(WireOp::Update);
    let parts = &mut duplicate_key.key.as_mut().expect("key").parts;
    parts.push(parts[0].clone());
    let mut missing_id = mutation(WireOp::Insert);
    missing_id.event_id.clear();
    let mut bad_record = mutation(WireOp::Insert);
    bad_record.record.as_mut().expect("record").fields[0] = field("id", Kind::SignedInteger(5));
    let mut duplicate_field = mutation(WireOp::Insert);
    duplicate_field
        .record
        .as_mut()
        .expect("record")
        .fields
        .push(field("id", Kind::SignedInteger(4)));
    let mut empty_key = mutation(WireOp::Upsert);
    empty_key.key.as_mut().expect("key").parts.clear();
    let mut bad_delete = mutation(WireOp::Insert);
    bad_delete.op = WireOp::Delete.into();
    let invalid = [
        Frame::Accepted(Default::default()),
        Frame::Mutation(foreign),
        Frame::Mutation(duplicate_key),
        Frame::Mutation(missing_id),
        Frame::Mutation(bad_record),
        Frame::Mutation(duplicate_field),
        Frame::Mutation(empty_key),
        Frame::Mutation(bad_delete),
        Frame::Mutation(mutation(WireOp::Unspecified)),
        Frame::SnapshotRow(Default::default()),
    ];
    let n_invalid = invalid.len();
    let frames = invalid
        .into_iter()
        .chain([
            Frame::ResnapshotRequired(Default::default()),
            Frame::Lagged(Default::default()),
        ])
        .map(Ok)
        .collect();
    let fixture = Fixture::new(frames, None).await;
    let mut events = fixture
        .source
        .watch(spec(), &subscriber())
        .await
        .expect("watch");
    for _ in 0..n_invalid {
        assert!(matches!(
            events.next().await,
            Some(Err(StreamError::InvalidData))
        ));
    }
    assert!(matches!(
        events.next().await,
        Some(Err(StreamError::ResnapshotRequired))
    ));
    assert!(matches!(
        events.next().await,
        Some(Err(StreamError::Lagged))
    ));
}

#[tokio::test]
async fn admission_and_midstream_status_reach_the_subscriber() {
    for code in [
        Code::Unauthenticated,
        Code::FailedPrecondition,
        Code::ResourceExhausted,
        Code::Unavailable,
    ] {
        let fixture = Fixture::new(vec![], Some(code)).await;
        let error = fixture
            .source
            .watch(spec(), &subscriber())
            .await
            .err()
            .expect("rejected");
        assert!(matches!(
            (code, error),
            (Code::Unauthenticated, StreamError::Denied)
                | (Code::FailedPrecondition, StreamError::ResnapshotRequired)
                | (Code::ResourceExhausted, StreamError::Lagged)
                | (Code::Unavailable, StreamError::Unavailable)
        ));
    }
    let fixture = Fixture::new(vec![Err(Status::out_of_range("expired"))], None).await;
    let mut events = fixture
        .source
        .watch(spec(), &subscriber())
        .await
        .expect("stream admitted");
    assert!(matches!(
        events.next().await,
        Some(Err(StreamError::ResnapshotRequired))
    ));
}

#[tokio::test]
async fn invalid_configuration_and_request_never_open_a_stream() {
    for (uri, projection) in [("not a uri", "approved"), ("http://localhost", "")] {
        let error = FabricChangeSource::new(
            FrfConfig {
                endpoint: uri.into(),
                projection: projection.into(),
            },
            None,
        )
        .err()
        .expect("invalid configuration");
        assert!(matches!(
            error,
            FabricError::Connect(_) | FabricError::Configuration(_)
        ));
    }
    let mut fixture = Fixture::new(vec![], None).await;
    for entity_type in ["widgets", "public.2widgets", "public.widgets.more"] {
        let mut invalid = spec();
        invalid.entity_type = entity_type.into();
        assert!(matches!(
            fixture.source.watch(invalid, &subscriber()).await,
            Err(StreamError::Denied)
        ));
    }
    let mut invalid = spec();
    invalid.tenant.clear();
    assert!(matches!(
        fixture.source.watch(invalid, &subscriber()).await,
        Err(StreamError::Denied)
    ));
    for checkpoint in ["%", "_w"] {
        let mut invalid = spec();
        invalid.checkpoint = Some(checkpoint.into());
        assert!(matches!(
            fixture.source.watch(invalid, &subscriber()).await,
            Err(StreamError::InvalidCheckpoint)
        ));
    }
    assert!(fixture.received.try_recv().is_err());
}
