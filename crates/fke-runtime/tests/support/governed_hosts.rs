use super::*;
use crate::host_bindings::flint::host::{db, identity, llm, secrets};
use async_trait::async_trait;
use fdb_ports::{BackendError, Conn, DatabaseBackend};
use forge_policy::Request;
use serde_json::{json, Value};
use wasmtime::component::Resource;

struct Backend;

#[async_trait]
impl DatabaseBackend for Backend {
    async fn acquire(&self, _: &RlsContext) -> Result<Conn, BackendError> {
        panic!("host queries must use the governed SQL port")
    }

    async fn query_json(
        &self,
        caller: &RlsContext,
        sql: &str,
        params: &[String],
    ) -> Result<Vec<String>, BackendError> {
        assert_eq!(caller.raw_bearer, "fixture-token");
        let values: Vec<Value> = params
            .iter()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        let expected = match sql {
            "SELECT $1::text AS title" => {
                assert_eq!(values, [json!("sample")]);
                return Ok(vec![json!({"title":"sample"}).to_string()]);
            }
            "SELECT llm.embed($1) AS result_value" => {
                assert_eq!(values, [json!("embed me")]);
                json!([0.25])
            }
            "SELECT llm.embed($1, $2) AS result_value" => {
                assert_eq!(values, [json!("embed me"), json!("local")]);
                json!([0.5])
            }
            "SELECT llm.complete($1, $2::jsonb) AS result_value" => {
                assert_eq!(values, [json!("sample"), json!({})]);
                json!("default response")
            }
            "SELECT llm.complete($1, $2::jsonb, $3) AS result_value" => {
                assert_eq!(
                    values,
                    [json!("sample"), json!({"model":"local"}), json!("local")]
                );
                json!("local response")
            }
            "SELECT vault.reveal_for_kiln($1, $2) AS result_value" => {
                assert_eq!(values, [json!("fixture"), json!("fixture-user")]);
                json!("fixture-value")
            }
            _ => return Err(BackendError::Denied),
        };
        Ok(vec![json!({"result_value":expected}).to_string()])
    }
}

struct AllowFixtureSecret;

#[async_trait]
impl Pep for AllowFixtureSecret {
    async fn check(&self, caller: &RlsContext, request: &Request) -> Decision {
        assert_eq!(caller.raw_bearer, "fixture-token");
        assert_eq!(request.action, "kiln:secret:reveal");
        assert_eq!(request.resource, "kiln:secret:fixture");
        Decision::Allow
    }
}

fn host() -> KilnHostState {
    KilnHostState {
        wasi_ctx: WasiCtxBuilder::new().build(),
        table: ResourceTable::new(),
        http_ctx: WasiHttpCtx::new(),
        http_hooks: [],
        granted: vec![],
        kv_store: HashMap::new(),
        identity: None,
        database: None,
        pep: None,
    }
}

fn caller() -> RlsContext {
    RlsContext {
        raw_bearer: "fixture-token".into(),
        role: "authenticated".into(),
        claims_json: r#"{"sub":"fixture-user"}"#.into(),
        keto_subject: "fixture-user".into(),
        vault_key_id: None,
    }
}

#[tokio::test]
async fn database_and_inference_use_the_invocation_identity_and_bind_parameters() {
    let mut state = host();
    async fn query(s: &mut KilnHostState) -> Result<Vec<String>, db::HostError> {
        db::Host::query(
            s,
            "SELECT $1::text AS title".into(),
            vec![json!("sample").to_string()],
        )
        .await
    }
    assert_eq!(
        query(&mut state).await.unwrap_err().code,
        "CAPABILITY_DENIED"
    );
    assert_eq!(
        llm::Host::embed(&mut state, "sample".into(), None)
            .await
            .unwrap_err()
            .code,
        "CAPABILITY_DENIED"
    );
    state.granted = vec![Capability::Db, Capability::Llm];
    assert_eq!(query(&mut state).await.unwrap_err().code, "UNAVAILABLE");
    state.database = Some(Arc::new(Backend));
    assert_eq!(query(&mut state).await.unwrap_err().code, "NO_IDENTITY");
    state.identity = Some(caller());
    let row: Value = serde_json::from_str(&query(&mut state).await.unwrap()[0]).unwrap();
    assert_eq!(row, json!({"title":"sample"}));
    assert_eq!(
        db::Host::query(&mut state, "SELECT restricted".into(), vec![])
            .await
            .unwrap_err()
            .code,
        "QUERY_FAILED"
    );
    for (model, expected) in [(None, 0.25), (Some("local".to_owned()), 0.5)] {
        assert_eq!(
            llm::Host::embed(&mut state, "embed me".into(), model)
                .await
                .unwrap(),
            vec![expected]
        );
    }
    for (opts, expected) in [
        (json!({}), "default response"),
        (json!({"model":"local"}), "local response"),
    ] {
        assert_eq!(
            llm::Host::complete(&mut state, "sample".into(), opts.to_string())
                .await
                .unwrap(),
            expected
        );
    }
}

#[tokio::test]
async fn secret_handles_require_a_fresh_grant_and_are_released_on_drop() {
    let mut state = host();
    assert_eq!(identity::Host::claims(&mut state), "{}");
    assert_eq!(
        secrets::Host::get(&mut state, "fixture".into())
            .await
            .unwrap_err()
            .code,
        "CAPABILITY_DENIED"
    );
    state.identity = Some(caller());
    assert_eq!(identity::Host::origin_jwt(&mut state), None);
    state.granted = vec![Capability::Secrets, Capability::Identity];
    assert_eq!(
        identity::Host::origin_jwt(&mut state).as_deref(),
        Some("fixture-token")
    );
    assert_eq!(identity::Host::claims(&mut state), caller().claims_json);
    let handle = secrets::Host::get(&mut state, "fixture".into())
        .await
        .unwrap();
    let borrowed = || Resource::new_borrow(handle.rep());
    assert_eq!(
        secrets::HostSecret::reveal(&mut state, borrowed())
            .await
            .unwrap_err()
            .code,
        "CEDAR_DENY"
    );
    state.pep = Some(Arc::new(AllowFixtureSecret));
    assert_eq!(
        secrets::HostSecret::reveal(&mut state, borrowed())
            .await
            .unwrap_err()
            .code,
        "UNAVAILABLE"
    );
    state.database = Some(Arc::new(Backend));
    assert_eq!(
        secrets::HostSecret::reveal(&mut state, borrowed())
            .await
            .unwrap(),
        "fixture-value"
    );
    state.pep = None;
    assert_eq!(
        secrets::HostSecret::reveal(&mut state, borrowed())
            .await
            .unwrap_err()
            .code,
        "CEDAR_DENY"
    );
    let rep = handle.rep();
    secrets::HostSecret::drop(&mut state, handle).await.unwrap();
    assert_eq!(
        secrets::HostSecret::reveal(&mut state, Resource::new_borrow(rep))
            .await
            .unwrap_err()
            .code,
        "INTERNAL"
    );
}
