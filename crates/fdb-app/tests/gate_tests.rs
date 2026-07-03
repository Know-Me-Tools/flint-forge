//! Gate tests for subscription RLS drop and Keto mutation gate (p3-c016).
//!
//! These tests use mocks only — no live Postgres, no Flint Realtime Fabric.
#![forbid(unsafe_code)]

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use fdb_app::{MutationError, Quarry};
use fdb_domain::{
    ChangeEvent, ChangeOp, ColumnMeta, RestQuery, RestResult, SubscriptionSpec, TableMeta,
};
use fdb_ports::{
    BackendError, ChangeStreamSource, GraphQlExecutor, KetoCheck, RestExecutor, StreamError,
};
use forge_domain::Json;
use forge_identity::RlsContext;
use futures::stream::{BoxStream, StreamExt};
use serde_json::json;

/// Mock Keto gate that allows or denies based on a flag.
struct MockKetoCheck {
    allow: bool,
}

#[async_trait]
impl KetoCheck for MockKetoCheck {
    async fn check(
        &self,
        _namespace: &str,
        _object: &str,
        _relation: &str,
        _subject: &str,
    ) -> bool {
        self.allow
    }
}

/// Mock REST executor returning configured rows per table.
struct MockRest {
    rows_by_table: HashMap<String, Json>,
}

#[async_trait]
impl RestExecutor for MockRest {
    async fn execute(
        &self,
        q: RestQuery,
        _: &RlsContext,
    ) -> Result<RestResult, BackendError> {
        let rows = self
            .rows_by_table
            .get(&q.table)
            .cloned()
            .unwrap_or_else(|| Json::Array(vec![]));
        let count = match &rows {
            Json::Array(a) => Some(a.len() as u64),
            _ => None,
        };
        Ok(RestResult { rows, count })
    }
}

/// Mock change stream source emitting a fixed event list.
struct MockChanges {
    events: Vec<ChangeEvent>,
}

#[async_trait]
impl ChangeStreamSource for MockChanges {
    async fn watch(
        &self,
        _: SubscriptionSpec,
        _: &RlsContext,
    ) -> Result<BoxStream<'static, Result<ChangeEvent, StreamError>>, StreamError> {
        let events = self.events.clone();
        Ok(futures::stream::iter(events.into_iter().map(Ok)).boxed())
    }
}

/// Stub GraphQL executor — never called by these tests.
struct StubGraphQl;

#[async_trait]
impl GraphQlExecutor for StubGraphQl {
    async fn execute(
        &self,
        _: fdb_domain::GraphQlRequest,
        _: &RlsContext,
    ) -> Result<Json, BackendError> {
        unreachable!("stub")
    }
}

fn make_rls(subject: &str) -> RlsContext {
    RlsContext {
        role: "authenticated".into(),
        claims_json: format!("{{\"sub\":\"{subject}\"}}"),
        raw_bearer: "bearer".into(),
        keto_subject: subject.into(),
        vault_key_id: None,
    }
}

fn make_quarry(rest: MockRest, changes: MockChanges) -> Quarry {
    Quarry::new(Arc::new(rest), Arc::new(StubGraphQl), Arc::new(changes))
}

fn orders_table_meta() -> TableMeta {
    TableMeta {
        schema: "public".into(),
        name: "orders".into(),
        columns: vec![ColumnMeta {
            name: "id".into(),
            sql_type: "bigint".into(),
            nullable: false,
        }],
        primary_key: vec!["id".into()],
        rls_enabled: true,
    }
}

#[tokio::test]
async fn test_subscription_rls_drops_unauthorized_events() {
    let event = ChangeEvent {
        op: ChangeOp::Insert,
        schema: "public".into(),
        table: "orders".into(),
        record: Some(json!({"id": 42})),
        old_record: None,
    };
    let changes = MockChanges {
        events: vec![event],
    };
    let rest = MockRest {
        rows_by_table: HashMap::new(),
    };
    let quarry = make_quarry(rest, changes);

    let stream = quarry
        .subscribe_rls_filtered(
            SubscriptionSpec {
                tenant: "t1".into(),
                entity_type: "orders".into(),
                filter: None,
            },
            orders_table_meta(),
            &make_rls("u1"),
        )
        .await
        .expect("subscribe should open the mock stream");

    let delivered: Vec<_> = stream.collect().await;
    assert!(
        delivered.is_empty(),
        "events that fail the RLS re-query must be silently dropped"
    );
}

#[tokio::test]
async fn test_keto_check_gates_mutation() {
    let quarry = make_quarry(
        MockRest {
            rows_by_table: HashMap::new(),
        },
        MockChanges { events: vec![] },
    )
    .with_keto(Arc::new(MockKetoCheck { allow: false }));

    let err = quarry
        .execute_rest_mutation(
            RestQuery {
                schema: "public".into(),
                table: "orders".into(),
                select: None,
                filters: vec![],
                order: None,
                limit: None,
                offset: None,
            },
            &make_rls("u1"),
        )
        .await
        .expect_err("denied keto check should fail the mutation");

    assert!(matches!(err, MutationError::Forbidden));
}

#[tokio::test]
async fn test_keto_allow_reaches_executor() {
    let quarry = make_quarry(
        MockRest {
            rows_by_table: [(
                "orders".into(),
                json!([{"id": 42, "status": "confirmed"}]),
            )]
            .into_iter()
            .collect(),
        },
        MockChanges { events: vec![] },
    )
    .with_keto(Arc::new(MockKetoCheck { allow: true }));

    let result = quarry
        .execute_rest_mutation(
            RestQuery {
                schema: "public".into(),
                table: "orders".into(),
                select: None,
                filters: vec![],
                order: None,
                limit: None,
                offset: None,
            },
            &make_rls("u1"),
        )
        .await
        .expect("allowed keto check should reach the executor");

    assert_eq!(
        result.rows,
        json!([{"id": 42, "status": "confirmed"}])
    );
}
