//! Subscription behavior at the change-source, REST and GraphQL boundaries.
use async_trait::async_trait;
use fdb_app::Quarry;
use fdb_domain::{
    ChangeEvent, ChangeOp, ColumnMeta, GraphQlRequest, RestQuery, RestResult, SubscriptionSpec,
    TableMeta,
};
use fdb_ports::{BackendError, ChangeStreamSource, GraphQlExecutor, RestExecutor, StreamError};
use forge_identity::RlsContext;
use futures::stream::{self, BoxStream, StreamExt};
use serde_json::{json, Value};
use std::sync::Arc;

struct Fixture;

fn table() -> TableMeta {
    TableMeta {
        schema: "public".into(),
        name: "notes".into(),
        columns: ["id", "caption"]
            .map(|name| ColumnMeta {
                name: name.into(),
                sql_type: "text".into(),
                nullable: false,
            })
            .to_vec(),
        primary_key: vec!["id".into()],
        rls_enabled: true,
    }
}

fn event(op: ChangeOp, id: Value) -> ChangeEvent {
    ChangeEvent {
        op,
        schema: "public".into(),
        table: "notes".into(),
        record: Some(Value::Object([("id".into(), id)].into_iter().collect())),
        old_record: None,
        event_id: Some("event".into()),
        checkpoint: Some("next".into()),
        projection: Some(vec!["caption".into(), "id".into()]),
        delete_authorized: false,
        truncated: true,
    }
}

fn identity() -> RlsContext {
    RlsContext {
        role: "authenticated".into(),
        claims_json: json!({"tenant_id":"test", "sub":"reader"}).to_string(),
        keto_subject: "reader".into(),
        raw_bearer: String::new(),
        vault_key_id: None,
    }
}

#[async_trait]
impl ChangeStreamSource for Fixture {
    async fn watch(
        &self,
        spec: SubscriptionSpec,
        who: &RlsContext,
    ) -> Result<BoxStream<'static, Result<ChangeEvent, StreamError>>, StreamError> {
        assert_eq!(spec.tenant, "test");
        assert_eq!(spec.entity_type, "public.notes");
        assert_eq!(who.keto_subject, "reader");
        if spec.checkpoint.is_some() {
            return Err(StreamError::InvalidCheckpoint);
        }
        let mut update = event(ChangeOp::Update, json!(2));
        update.projection = None;
        let mut deleted = event(ChangeOp::Delete, Value::Null);
        deleted.record = None;
        deleted.old_record = Some(json!({"id":"old", "caption":"old caption"}));
        deleted.delete_authorized = true;
        let mut foreign = update.clone();
        foreign.table = "other".into();
        let mut wrong_projection = update.clone();
        wrong_projection.projection = Some(vec!["removed_column".into()]);
        Ok(stream::iter([
            Ok(event(ChangeOp::Insert, json!("new"))),
            Ok(update),
            Ok(event(ChangeOp::Upsert, json!(true))),
            Ok(deleted),
            Ok(event(ChangeOp::Insert, json!("hidden"))),
            Ok(event(ChangeOp::Update, json!("offline"))),
            Ok(foreign),
            Ok(wrong_projection),
            Ok(event(ChangeOp::Update, Value::Null)),
            Err(StreamError::Unavailable),
            Err(StreamError::Denied),
            Err(StreamError::InvalidCheckpoint),
            Err(StreamError::ResnapshotRequired),
            Err(StreamError::Lagged),
            Err(StreamError::InvalidData),
        ])
        .boxed())
    }
}

#[async_trait]
impl RestExecutor for Fixture {
    async fn execute(
        &self,
        query: RestQuery,
        who: &RlsContext,
    ) -> Result<RestResult, BackendError> {
        assert_eq!(who.keto_subject, "reader");
        assert_eq!(
            (query.schema.as_str(), query.table.as_str()),
            ("public", "notes")
        );
        assert_eq!(query.limit, Some(1));
        let [(column, op, value)] = query.filters.as_slice() else {
            panic!("expected primary-key equality")
        };
        assert_eq!((column.as_str(), op.as_str()), ("id", "eq"));
        assert_eq!(
            query.select.as_deref(),
            Some(if value == "2" {
                "id,caption"
            } else {
                "caption,id"
            })
        );
        let rows = match value.as_str() {
            "hidden" => json!([]),
            "offline" => return Err(BackendError::Connection),
            "new" | "2" | "true" => json!([{"id":value, "caption":null}]),
            other => panic!("unexpected re-query for {other}"),
        };
        Ok(RestResult { rows, count: None })
    }
}

#[async_trait]
impl GraphQlExecutor for Fixture {
    async fn execute(&self, _: GraphQlRequest, _: &RlsContext) -> Result<Value, BackendError> {
        panic!("subscriptions use the change and REST ports")
    }
}

#[tokio::test]
async fn subscriptions_project_visible_rows_and_surface_stable_stream_errors() {
    let port = Arc::new(Fixture);
    let quarry = Quarry::new(port.clone(), port.clone(), port);
    let spec = SubscriptionSpec {
        tenant: "test".into(),
        entity_type: "public.notes".into(),
        checkpoint: None,
        filter: None,
    };
    let who = identity();
    let mut events = quarry
        .subscribe_graphql_values(spec.clone(), table(), &who)
        .await
        .expect("subscribe");
    for (op, id) in [
        ("insert", "new"),
        ("update", "2"),
        ("upsert", "true"),
        ("delete", "old"),
    ] {
        let value = events
            .next()
            .await
            .expect("event")
            .expect("visible row")
            .into_json()
            .expect("JSON object");
        assert_eq!(value["_op"], op);
        assert_eq!(value["id"], id);
        assert_eq!(value["_eventId"], "event");
        assert_eq!(value["_checkpoint"], "next");
        if op == "delete" {
            assert!(value.get("caption").is_none());
        } else {
            assert_eq!(value.get("caption"), Some(&Value::Null));
        }
    }
    for code in [
        "RLS_REQUERY_FAILED",
        "INVALID_STREAM_DATA",
        "INVALID_STREAM_DATA",
        "INVALID_STREAM_DATA",
        "SOURCE_UNAVAILABLE",
        "AUTHZ_DENIED",
        "INVALID_CHECKPOINT",
        "RESNAPSHOT_REQUIRED",
        "SUBSCRIBER_LAGGED",
        "INVALID_STREAM_DATA",
    ] {
        let error = events
            .next()
            .await
            .expect("error frame")
            .expect_err("typed subscription error");
        assert_eq!(
            error.extensions.expect("error extensions").get("code"),
            Some(&async_graphql::Value::from(code))
        );
    }
    assert!(events.next().await.is_none());
    let resume = SubscriptionSpec {
        checkpoint: Some("expired".into()),
        ..spec
    };
    assert!(matches!(
        quarry.subscribe_graphql_values(resume, table(), &who).await,
        Err(fdb_app::SubscriptionError::Stream(
            StreamError::InvalidCheckpoint
        ))
    ));
}
