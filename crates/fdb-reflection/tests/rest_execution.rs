//! Exercise reflected endpoints through Axum and inspect the bound SQL executor calls.
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::{Extension, Router};
use fdb_ports::{BackendError, IdempotencyContext, SqlExecutor};
use fdb_query::QueryParam;
use fdb_reflection::{compilers::rest::RestCompiler, DatabaseModel};
use forge_identity::RlsContext;
use serde_json::{json, Map, Value};
use tower::ServiceExt;

type Rows = Vec<Map<String, Value>>;

#[derive(Default)]
struct Executor {
    replies: Mutex<VecDeque<Result<Rows, BackendError>>>,
    mutations: Mutex<Vec<IdempotencyContext>>,
    calls: Mutex<Vec<(String, Vec<QueryParam>)>>,
}

impl Executor {
    fn reply(&self, result: Result<Rows, BackendError>) {
        self.replies.lock().expect("replies").push_back(result);
    }
}

#[async_trait::async_trait]
impl SqlExecutor for Executor {
    async fn execute_raw(
        &self,
        sql: &str,
        params: Vec<QueryParam>,
        rls: &RlsContext,
    ) -> Result<Rows, BackendError> {
        assert_eq!(rls.subject().expect("request subject").0, "reader");
        self.calls.lock().expect("calls").push((sql.into(), params));
        self.replies
            .lock()
            .expect("replies")
            .pop_front()
            .expect("expected query")
    }

    async fn execute_idempotent(
        &self,
        sql: &str,
        params: Vec<QueryParam>,
        rls: &RlsContext,
        ctx: &IdempotencyContext,
    ) -> Result<Rows, BackendError> {
        self.mutations.lock().expect("mutations").push(ctx.clone());
        self.execute_raw(sql, params, rls).await
    }
}

fn router(executor: &Arc<Executor>) -> Router {
    let model: DatabaseModel = serde_json::from_value(json!({
        "version":1, "views":[],
        "tables":[{"schema":"public","name":"items","pk":["id"],"fk":[],"rls_enabled":true,
            "columns":[{"name":"id","pg_type":"integer","nullable":false},
                       {"name":"label","pg_type":"text","nullable":true}]}],
        "functions":[{"schema":"public","name":"nearest","return_type":"record","security_definer":false,
            "args":[{"name":"embedding","pg_type":"vector(2)"},{"name":"label","pg_type":"text"},
                    {"name":"options","pg_type":"jsonb"}]}]
    })).expect("reflected model");
    RestCompiler::compile(&model, executor.clone()).layer(Extension(RlsContext {
        role: "authenticated".into(),
        claims_json: r#"{"sub":"reader"}"#.into(),
        raw_bearer: "fixture".into(),
        keto_subject: "reader".into(),
        vault_key_id: None,
    }))
}

async fn request(
    app: &Router,
    method: &str,
    path: &str,
    body: Value,
    key: Option<&str>,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(key) = key {
        request = request.header("Idempotency-Key", key);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::from(body.to_string())).expect("request"))
        .await
        .expect("route");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 65_536)
        .await
        .expect("response body");
    let payload = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("JSON response")
    };
    (status, payload)
}

#[tokio::test]
async fn listing_and_creation_describe_the_returned_rows_in_headers() {
    let executor = Arc::new(Executor::default());
    let app = router(&executor);
    let row = json!({"id": 4, "label": "tea"});
    executor.reply(Ok(vec![Map::from_iter([
        ("rows".into(), json!([row])),
        ("total".into(), json!(8)),
    ])]));
    let list = app
        .clone()
        .oneshot(
            Request::get("/public/items")
                .header(header::RANGE, "rows=3-5")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("list");
    assert_eq!(list.status(), StatusCode::OK);
    assert_eq!(list.headers()[header::CONTENT_RANGE], "rows 3-3/8");
    let bytes = to_bytes(list.into_body(), 1024).await.expect("rows");
    assert_eq!(
        serde_json::from_slice::<Value>(&bytes).expect("row array"),
        json!([row])
    );
    assert_eq!(
        executor.calls.lock().expect("calls")[0].1,
        [QueryParam::BigInt(3), QueryParam::BigInt(3)]
    );

    for (created, location) in [
        (row, "/public/items/4"),
        (json!({"label":"tea"}), "/public/items"),
    ] {
        executor.reply(Ok(vec![Map::from_iter([("row".into(), created.clone())])]));
        let insert = app
            .clone()
            .oneshot(
                Request::post("/public/items")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"label":"tea"}"#))
                    .expect("request"),
            )
            .await
            .expect("insert");
        assert_eq!(insert.status(), StatusCode::CREATED);
        assert_eq!(insert.headers()[header::LOCATION], location);
        let bytes = to_bytes(insert.into_body(), 1024)
            .await
            .expect("created row");
        assert_eq!(
            serde_json::from_slice::<Value>(&bytes).expect("row"),
            created
        );
    }
}

#[tokio::test]
async fn rpc_binds_declared_argument_order_and_mixed_types() {
    let executor = Arc::new(Executor::default());
    let app = router(&executor);
    let rpc = "/rpc/public/nearest";
    let (status, body) = request(&app, "POST", rpc, json!({"label":"missing vector"}), None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "vector arg must be a JSON array, got null");
    executor.reply(Ok(vec![Map::from_iter([(
        "row".into(),
        json!({"item":"tea"}),
    )])]));
    let (status, body) = request(
        &app,
        "POST",
        rpc,
        json!({"options":{"top":1},"label":"drinks","embedding":[1,0.5]}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!([{"item":"tea"}]));
    assert_eq!(
        *executor.calls.lock().expect("calls"),
        [(
            "SELECT row_to_json(t) AS row FROM public.nearest($1::vector, $2, $3) AS t".into(),
            vec![
                QueryParam::Vector(vec![1.0, 0.5]),
                QueryParam::Text("drinks".into()),
                QueryParam::Json(r#"{"top":1}"#.into())
            ],
        )]
    );
}

#[tokio::test]
async fn keyed_mutations_bind_request_identity_to_action_body_and_filter() {
    let executor = Arc::new(Executor::default());
    let app = router(&executor);
    let key = Some("receipt-42");
    for (method, path, body) in [
        (
            "PATCH",
            "/public/items?label=eq.old&id=eq.1",
            json!({"label":"new","id":1}),
        ),
        (
            "PATCH",
            "/public/items?id=eq.1&label=eq.old",
            json!({"id":1,"label":"new"}),
        ),
        (
            "PATCH",
            "/public/items?id=eq.2&label=eq.old",
            json!({"id":1,"label":"new"}),
        ),
        (
            "PATCH",
            "/public/items?id=eq.1&label=eq.old",
            json!({"id":1,"label":"other"}),
        ),
        ("DELETE", "/public/items?id=eq.1&label=eq.old", Value::Null),
    ] {
        executor.reply(Ok(vec![]));
        let (status, _) = request(&app, method, path, body, key).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }
    let receipts = executor.mutations.lock().expect("mutations").clone();
    assert_eq!(receipts.len(), 5);
    assert!(receipts
        .iter()
        .all(|receipt| receipt.mutation_id == "receipt-42"));
    assert_eq!(receipts[0], receipts[1]);
    assert!(receipts[2..]
        .iter()
        .all(|receipt| receipt.request_fingerprint != receipts[0].request_fingerprint));

    for method in ["POST", "PATCH", "DELETE"] {
        let (status, payload) = request(
            &app,
            method,
            "/public/items",
            json!({"label":"new"}),
            Some("invalid key"),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(payload["error"], "invalid Idempotency-Key header");
    }
    assert_eq!(executor.calls.lock().expect("calls").len(), 5);
}

#[tokio::test]
async fn mutations_translate_backend_failures_into_appropriate_public_responses() {
    let executor = Arc::new(Executor::default());
    let app = router(&executor);
    for method in ["POST", "PATCH", "DELETE"] {
        for (failure, status, message) in [
            (BackendError::Denied, StatusCode::FORBIDDEN, "forbidden"),
            (
                BackendError::ConstraintConflict,
                StatusCode::CONFLICT,
                "mutation conflicts with existing data",
            ),
            (
                BackendError::IdempotencyConflict,
                StatusCode::CONFLICT,
                "idempotency key was already used for a different mutation",
            ),
            (
                BackendError::Internal("private diagnostic".into()),
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error",
            ),
        ] {
            executor.reply(Err(failure));
            let actual = request(
                &app,
                method,
                "/public/items?id=eq.1",
                json!({"label":"new"}),
                Some("retry"),
            )
            .await;
            assert_eq!(actual, (status, json!({"error":message})));
        }
    }
    for (method, path, body) in [
        ("GET", "/public/items", Value::Null),
        ("POST", "/rpc/public/nearest", json!({"embedding":[0,1]})),
    ] {
        executor.reply(Err(BackendError::Connection));
        assert_eq!(
            request(&app, method, path, body, None).await,
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error":"internal server error"})
            )
        );
    }
}
