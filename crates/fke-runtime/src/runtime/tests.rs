use super::*;
use async_trait::async_trait;
use forge_identity::RlsContext;
use forge_policy::{Decision, Pep, Request};

struct AllowAll;
struct DenyAll;

#[async_trait]
impl Pep for AllowAll {
    async fn check(&self, _who: &RlsContext, _req: &Request) -> Decision {
        Decision::Allow
    }
}

#[async_trait]
impl Pep for DenyAll {
    async fn check(&self, _who: &RlsContext, _req: &Request) -> Decision {
        Decision::Deny
    }
}

/// Allows `kiln:invoke` (so the outer Cedar gate passes) but denies the
/// `kiln:capability:db` action specifically — everything else is
/// allowed. Used to prove the capability check is a REAL per-capability
/// Cedar comparison, not the historical `check_capabilities(granted,
/// granted)` no-op.
struct AllowInvokeDenyDbCapability;

#[async_trait]
impl Pep for AllowInvokeDenyDbCapability {
    async fn check(&self, _who: &RlsContext, req: &Request) -> Decision {
        if req.action == "kiln:capability:db" {
            Decision::Deny
        } else {
            Decision::Allow
        }
    }
}

fn fake_rls() -> RlsContext {
    RlsContext {
        role: "authenticated".into(),
        claims_json: r#"{"sub":"test"}"#.into(),
        raw_bearer: "fake".into(),
        keto_subject: "test".into(),
        vault_key_id: None,
    }
}

fn dummy_request() -> KilnRequest {
    KilnRequest {
        method: "POST".into(),
        uri: "/functions/v1/test".into(),
        headers: vec![],
        body: vec![],
    }
}

#[tokio::test]
async fn edge_runtime_constructs() {
    EdgeRuntime::new().expect("construct");
}

#[tokio::test]
async fn fuel_override_works() {
    let rt = EdgeRuntime::new().expect("construct").with_fuel(500_000);
    assert_eq!(rt.fuel_per_call, 500_000);
}

#[tokio::test]
async fn load_wasm_rejects_garbage() {
    let rt = EdgeRuntime::new().expect("construct");
    let id = ContentId("sha256:deadbeef".into());
    assert!(rt.load_wasm(id, b"not valid wasm").is_err());
}

#[tokio::test]
async fn no_pep_skips_cedar_check() {
    let rt = EdgeRuntime::new().expect("construct");
    let id = ContentId("sha256:notloaded".into());
    let err = rt
        .handle(&id, &[], None, dummy_request())
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("not loaded"),
        "expected 'not loaded' error, got: {err}"
    );
}

#[tokio::test]
async fn deny_all_pep_with_caller_returns_policy_denied() {
    let rt = EdgeRuntime::new()
        .expect("construct")
        .with_pep(Arc::new(DenyAll));
    let id = ContentId("sha256:any".into());
    let who = fake_rls();
    let err = rt
        .handle(&id, &[], Some(&who), dummy_request())
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("policy denied"),
        "expected 'policy denied', got: {err}"
    );
}

#[tokio::test]
async fn deny_all_pep_without_caller_falls_through_to_runtime() {
    let rt = EdgeRuntime::new()
        .expect("construct")
        .with_pep(Arc::new(DenyAll));
    let id = ContentId("sha256:notloaded".into());
    let err = rt
        .handle(&id, &[], None, dummy_request())
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("not loaded"),
        "expected 'not loaded' (Cedar skipped), got: {err}"
    );
}

#[tokio::test]
async fn allow_all_pep_with_caller_falls_through_to_runtime() {
    let rt = EdgeRuntime::new()
        .expect("construct")
        .with_pep(Arc::new(AllowAll));
    let id = ContentId("sha256:notloaded".into());
    let who = fake_rls();
    let err = rt
        .handle(&id, &[], Some(&who), dummy_request())
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("not loaded"),
        "expected 'not loaded', got: {err}"
    );
}

#[test]
fn check_capabilities_passes_when_all_granted() {
    let granted = vec![Capability::Db, Capability::Llm];
    assert!(check_capabilities(&[Capability::Db], &granted).is_ok());
}

#[test]
fn check_capabilities_fails_on_missing() {
    let granted = vec![Capability::Db];
    let err = check_capabilities(&[Capability::Db, Capability::HttpOutgoing], &granted);
    assert!(err.is_err());
    assert!(err.unwrap_err().to_string().contains("HttpOutgoing"));
}

#[test]
fn check_capabilities_empty_required_always_passes() {
    assert!(check_capabilities(&[], &[]).is_ok());
}

/// p16-c003 gate: a component requesting a capability Cedar denies is
/// rejected BEFORE instantiate (never reaches the cache lookup) — not
/// the historical no-op that compared `granted` to itself and could
/// never fail.
#[tokio::test]
async fn ungranted_capability_denied_before_instantiate() {
    let rt = EdgeRuntime::new()
        .expect("construct")
        .with_pep(Arc::new(AllowInvokeDenyDbCapability));
    let id = ContentId("sha256:notloaded".into());
    let who = fake_rls();
    let err = rt
        .handle(&id, &[Capability::Db], Some(&who), dummy_request())
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("Db"),
        "expected a capability-denial error mentioning Db, got: {err}"
    );
    assert!(
        !err.to_string().contains("not loaded"),
        "must fail at the capability gate, before ever reaching the cache lookup; got: {err}"
    );
}

/// p16-c003 gate (no-regression half): a component requesting only
/// capabilities Cedar grants passes the check and proceeds to the
/// runtime as before (reaching the cache-miss error, since nothing is
/// actually loaded in this unit test).
#[tokio::test]
async fn granted_capability_passes_check_and_reaches_runtime() {
    let rt = EdgeRuntime::new()
        .expect("construct")
        .with_pep(Arc::new(AllowInvokeDenyDbCapability));
    let id = ContentId("sha256:notloaded".into());
    let who = fake_rls();
    let err = rt
        .handle(&id, &[Capability::Kv], Some(&who), dummy_request())
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("not loaded"),
        "granted capability must pass the check and reach the cache lookup; got: {err}"
    );
}

/// Gate test: load the hello-component WASM and verify it returns HTTP 200.
/// Requires the component to be pre-built with `cargo component build -p hello-component`.
#[tokio::test]
async fn gate_hello_component_returns_http_200() {
    let wasm_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/wasm32-wasip1/debug/hello_component.wasm"
    );
    let Ok(wasm_bytes) = std::fs::read(wasm_path) else {
        // Component not built yet — skip.
        eprintln!("hello_component.wasm not found — run `cargo component build -p hello-component` to enable this test");
        return;
    };

    let rt = EdgeRuntime::new().expect("construct");
    let id = ContentId("sha256:hello-component-test".into());
    rt.load_wasm(id.clone(), &wasm_bytes).expect("load_wasm");

    let req = KilnRequest {
        method: "GET".into(),
        uri: "/".into(),
        headers: vec![("host".into(), "localhost".into())],
        body: vec![],
    };
    let resp = rt.handle(&id, &[], None, req).await.expect("handle");

    assert_eq!(resp.status, 200, "expected HTTP 200 from hello-component");
    let body = String::from_utf8_lossy(&resp.body);
    assert!(
        body.contains("Hello") || !body.is_empty(),
        "expected non-empty body, got: {body:?}"
    );
}

/// `is_loaded` tracks cache membership without needing a live invocation.
#[tokio::test]
async fn is_loaded_reflects_cache_state() {
    let rt = EdgeRuntime::new().expect("construct");
    let present = ContentId("sha256:cache-present".into());
    let missing = ContentId("sha256:cache-missing".into());

    assert!(
        !rt.is_loaded(&present),
        "unloaded component must report false"
    );

    let wasm_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/wasm32-wasip1/debug/hello_component.wasm"
    );
    let Ok(wasm_bytes) = std::fs::read(wasm_path) else {
        eprintln!("hello_component.wasm not found — skipping cache-state test");
        return;
    };

    rt.load_wasm(present.clone(), &wasm_bytes)
        .expect("load valid wasm");
    assert!(rt.is_loaded(&present), "loaded component must report true");
    assert!(
        !rt.is_loaded(&missing),
        "different id must still report false"
    );
}

/// Epoch ticker is spawned when constructed with default settings.
#[tokio::test]
async fn epoch_ticker_spawned_by_default() {
    let rt = EdgeRuntime::new().expect("construct");
    assert!(
        rt.epoch_ticker.is_some(),
        "expected epoch ticker with default 10ms interval"
    );
}

/// Fast component still returns HTTP 200 under aggressive epoch ticking
/// (uses default 10ms interval — a fast component must complete before any tick fires).
#[tokio::test]
async fn gate_hello_component_survives_fast_epoch_ticks() {
    let wasm_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/wasm32-wasip1/debug/hello_component.wasm"
    );
    let Ok(wasm_bytes) = std::fs::read(wasm_path) else {
        eprintln!("hello_component.wasm not found — skipping epoch tick gate test");
        return;
    };
    let rt = EdgeRuntime::new().expect("construct");
    let id = ContentId("sha256:hello-component-epoch-test".into());
    rt.load_wasm(id.clone(), &wasm_bytes).expect("load_wasm");

    let req = KilnRequest {
        method: "GET".into(),
        uri: "/".into(),
        headers: vec![("host".into(), "localhost".into())],
        body: vec![],
    };
    let resp = rt
        .handle(&id, &[], None, req)
        .await
        .expect("fast component must complete before epoch deadline");

    assert_eq!(resp.status, 200, "expected HTTP 200 under epoch ticking");
}
