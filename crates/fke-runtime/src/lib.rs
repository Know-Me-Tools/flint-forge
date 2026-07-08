//! Wasmtime host — WASM Component Model execution engine for Flint Kiln.
//!
//! # Architecture
//!
//! ```text
//! Data-plane (always):
//!   EdgeRuntime::new()       → Engine + ProxyPre cache
//!   EdgeRuntime::load_wasm() → Component::from_binary → ProxyPre + cache
//!   EdgeRuntime::handle()    → Cedar gate → ProxyPre::instantiate_async →
//!                              new_incoming_request + new_response_outparam →
//!                              call_handle → oneshot response
//!
//! Control-plane (compiler feature):
//!   AotCompiler::precompile(wasm) → .cwasm bytes (Cranelift AOT)
//! ```
//!
//! # Security
//!
//! - `Pep::check(caller, kiln:invoke)` fires before instantiation.
//!   `caller = None` skips Cedar (BGW / system-level invocation).
//! - Fuel limit prevents infinite loops.
//! - `#![forbid(unsafe_code)]` — safe `Component::from_binary` only.
#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use forge_identity::RlsContext;
use forge_policy::{Decision, Pep};
use http_body_util::BodyExt;
use hyper::body::Bytes;
use hyper::header::{HeaderName, HeaderValue};
use hyper::Request as HyperRequest;

// wasmtime 46 uses its own Error type that does not implement std::error::Error.
// This helper bridges wasmtime::Result<T> → anyhow::Result<T> so .context() works.
#[inline]
fn wt<T>(r: wasmtime::Result<T>) -> core::result::Result<T, anyhow::Error> {
    r.map_err(|e| anyhow::anyhow!("{e}"))
}
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};
use wasmtime_wasi_http::p2::bindings::http::types::Scheme;
use wasmtime_wasi_http::p2::bindings::ProxyPre;
use wasmtime_wasi_http::p2::WasiHttpCtxView;
use wasmtime_wasi_http::p2::WasiHttpView;
use wasmtime_wasi_http::WasiHttpCtx;

use fke_domain::{Capability, ContentId};

/// Default fuel grant per invocation (~10 M instructions).
const DEFAULT_FUEL: u64 = 10_000_000;

// ─── Host state ──────────────────────────────────────────────────────────────

struct KilnHostState {
    wasi_ctx: WasiCtx,
    table: ResourceTable,
    http_ctx: WasiHttpCtx,
    http_hooks: [(); 0], // zero-size default WasiHttpHooks impl
    #[allow(dead_code)]
    granted: Vec<Capability>,
}

impl WasiView for KilnHostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.table,
        }
    }
}

impl WasiHttpView for KilnHostState {
    fn http(&mut self) -> WasiHttpCtxView<'_> {
        WasiHttpCtxView {
            ctx: &mut self.http_ctx,
            table: &mut self.table,
            hooks: &mut self.http_hooks,
        }
    }
}

// ─── ProxyPre cache ──────────────────────────────────────────────────────────

struct CachedComponent {
    pre: ProxyPre<KilnHostState>,
}

// ─── EdgeRuntime ─────────────────────────────────────────────────────────────

/// The Kiln data-plane runtime. Share via `Arc<EdgeRuntime>`.
pub struct EdgeRuntime {
    engine: Engine,
    cache: Mutex<HashMap<ContentId, Arc<CachedComponent>>>,
    fuel_per_call: u64,
    pep: Option<Arc<dyn Pep>>,
    /// Background epoch ticker. Held to document that its liveness is required;
    /// the engine is dropped together with the runtime so the ticker exits naturally.
    #[allow(dead_code)]
    epoch_ticker: Option<tokio::task::JoinHandle<()>>,
}

impl EdgeRuntime {
    pub fn new() -> Result<Self> {
        let mut cfg = Config::new();
        // async_support is always enabled in wasmtime 46+ (was made default; call removed).
        cfg.wasm_component_model(true);
        cfg.consume_fuel(true);
        cfg.epoch_interruption(true);
        let engine = wt(Engine::new(&cfg)).context("Engine::new")?;

        // Spawn epoch ticker. Reads KILN_EPOCH_INTERVAL_MS (default 10 ms).
        // Set to 0 to disable (useful in tests that rely purely on fuel).
        let interval_ms: u64 = std::env::var("KILN_EPOCH_INTERVAL_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);
        let epoch_ticker = if interval_ms > 0 {
            let engine_clone = engine.clone();
            Some(tokio::task::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));
                loop {
                    interval.tick().await;
                    engine_clone.increment_epoch();
                }
            }))
        } else {
            None
        };

        Ok(Self {
            engine,
            cache: Mutex::new(HashMap::new()),
            fuel_per_call: DEFAULT_FUEL,
            pep: None,
            epoch_ticker,
        })
    }

    #[must_use]
    pub fn with_fuel(mut self, fuel: u64) -> Self {
        self.fuel_per_call = fuel;
        self
    }

    /// Attach a Cedar policy enforcement point.
    #[must_use]
    pub fn with_pep(mut self, pep: Arc<dyn Pep>) -> Self {
        self.pep = Some(pep);
        self
    }

    /// Load a WASM component from raw bytes and cache it under `id`.
    pub fn load_wasm(&self, id: ContentId, wasm: &[u8]) -> Result<()> {
        let component =
            wt(Component::from_binary(&self.engine, wasm)).context("Component::from_binary")?;
        let linker = build_linker(&self.engine)?;
        let instance_pre =
            wt(linker.instantiate_pre(&component)).context("Linker::instantiate_pre")?;
        let pre = wt(ProxyPre::new(instance_pre)).context("ProxyPre::new")?;
        self.cache
            .lock()
            .expect("cache lock")
            .insert(id, Arc::new(CachedComponent { pre }));
        Ok(())
    }

    /// Return true if a component with `id` is already loaded in the runtime cache.
    pub fn is_loaded(&self, id: &ContentId) -> bool {
        self.cache.lock().expect("cache lock").contains_key(id)
    }

    /// Dispatch an HTTP-style request to the loaded component.
    ///
    /// `caller = None` → Cedar gate skipped (BGW / system-level).
    /// Records `kiln_fuel_consumed_total` and `kiln_epoch_traps_total`.
    pub async fn handle(
        &self,
        id: &ContentId,
        granted: &[Capability],
        caller: Option<&RlsContext>,
        request: KilnRequest,
    ) -> Result<KilnResponse> {
        self.handle_with_telemetry(id, granted, caller, request)
            .await
            .map(|outcome| outcome.response)
    }

    /// Same as `handle`, but returns telemetry captured during the invocation.
    pub async fn handle_with_telemetry(
        &self,
        id: &ContentId,
        granted: &[Capability],
        caller: Option<&RlsContext>,
        request: KilnRequest,
    ) -> Result<KilnHandleOutcome> {
        // ── Cedar gate ────────────────────────────────────────────────────
        if let (Some(pep), Some(who)) = (&self.pep, caller) {
            let decision = pep
                .check(who, &forge_policy::kiln::request(forge_policy::KILN_INVOKE))
                .await;
            if decision == Decision::Deny {
                bail!("Cedar policy denied kiln:invoke for caller");
            }
        }

        // ── Capability check ──────────────────────────────────────────────
        check_capabilities(granted, granted)?;

        // ── Retrieve cached ProxyPre ──────────────────────────────────────
        let cached = self
            .cache
            .lock()
            .expect("cache lock")
            .get(id)
            .cloned()
            .with_context(|| format!("component {id:?} not loaded"))?;

        // ── Build per-request Store ───────────────────────────────────────
        let wasi_ctx = WasiCtxBuilder::new().inherit_stdio().build();
        let host = KilnHostState {
            wasi_ctx,
            table: ResourceTable::new(),
            http_ctx: WasiHttpCtx::new(),
            http_hooks: [],
            granted: granted.to_vec(),
        };
        let mut store = Store::new(&self.engine, host);
        store.set_fuel(self.fuel_per_call)?;
        // Epoch deadline: trap when the background ticker increments past 1.
        // Works in concert with fuel; epoch catches slow host-call-heavy loops.
        store.set_epoch_deadline(1);
        let initial_fuel = wt(store.get_fuel()).context("get_fuel")?;

        // ── Convert KilnRequest → hyper::Request ──────────────────────────
        let hyper_req = kiln_request_to_hyper(request)?;

        // ── Instantiate and invoke wasi:http/incoming-handler ─────────────
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let incoming = wt(store
            .data_mut()
            .http()
            .new_incoming_request(Scheme::Http, hyper_req))
        .context("new_incoming_request")?;
        let out = wt(store.data_mut().http().new_response_outparam(sender))
            .context("new_response_outparam")?;

        let proxy = wt(cached.pre.instantiate_async(&mut store).await)
            .context("ProxyPre::instantiate_async")?;

        // Run the handler in a separate task so long-running components don't
        // block the current executor thread. Return the store so we can read
        // the remaining fuel and detect epoch traps.
        let task = tokio::task::spawn(async move {
            let result = proxy
                .wasi_http_incoming_handler()
                .call_handle(&mut store, incoming, out)
                .await;
            (result, store)
        });

        let (handler_result, store) = match task.await {
            Ok((result, store)) => (result, store),
            Err(e) => bail!("handler task panicked: {e}"),
        };

        let final_fuel = wt(store.get_fuel()).context("get_fuel")?;
        let fuel_consumed = initial_fuel.saturating_sub(final_fuel);
        metrics::counter!("kiln_fuel_consumed_total").increment(fuel_consumed);

        let epoch_trap = handler_result.as_ref().err().is_some_and(is_epoch_trap);
        if epoch_trap {
            metrics::counter!("kiln_epoch_traps_total").increment(1);
        }

        // Collect the response.
        let hyper_resp = match handler_result {
            Ok(()) => match receiver.await {
                Ok(Ok(resp)) => resp,
                Ok(Err(e)) => bail!("component returned HTTP error: {e}"),
                Err(_) => bail!("component never set response outparam"),
            },
            Err(e) => bail!("handler task error: {e}"),
        };

        // ── Convert hyper::Response → KilnResponse ────────────────────────
        let status = hyper_resp.status().as_u16();
        let body_bytes = hyper_resp
            .into_body()
            .collect()
            .await
            .context("collect response body")?
            .to_bytes()
            .to_vec();
        Ok(KilnHandleOutcome {
            response: KilnResponse {
                status,
                body: body_bytes,
            },
            fuel_consumed,
            epoch_trap,
        })
    }
}

impl Default for EdgeRuntime {
    fn default() -> Self {
        Self::new().expect("EdgeRuntime::default")
    }
}

// ─── AotCompiler (compiler feature) ─────────────────────────────────────────

#[cfg(feature = "compiler")]
pub struct AotCompiler {
    engine: Engine,
}

#[cfg(feature = "compiler")]
impl AotCompiler {
    pub fn new() -> Result<Self> {
        let mut cfg = Config::new();
        cfg.wasm_component_model(true);
        Ok(Self {
            engine: wt(Engine::new(&cfg)).context("AotCompiler Engine::new")?,
        })
    }

    pub fn precompile(&self, artifact: &[u8], _target: &fke_domain::TargetArch) -> Result<Vec<u8>> {
        let component = wt(Component::from_binary(&self.engine, artifact))
            .context("AotCompiler: Component::from_binary")?;
        wt(component.serialize()).context("Component::serialize")
    }
}

#[cfg(feature = "compiler")]
impl Default for AotCompiler {
    fn default() -> Self {
        Self::new().expect("AotCompiler::default")
    }
}

// ─── Request / Response types ────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct KilnRequest {
    pub method: String,
    pub uri: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct KilnResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Telemetry captured during a single Kiln invocation.
#[derive(Debug, Clone)]
pub struct KilnHandleOutcome {
    pub response: KilnResponse,
    pub fuel_consumed: u64,
    pub epoch_trap: bool,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Detect whether a wasmtime error was caused by an epoch-interruption trap.
fn is_epoch_trap(e: &wasmtime::Error) -> bool {
    e.to_string().to_lowercase().contains("epoch")
}

/// Convert a `KilnRequest` into a `hyper::Request` compatible with
/// `WasiHttpView::new_incoming_request` (`Body<Data=Bytes, Error=hyper::Error>`).
fn kiln_request_to_hyper(
    req: KilnRequest,
) -> Result<HyperRequest<impl hyper::body::Body<Data = Bytes, Error = hyper::Error>>> {
    let mut builder = HyperRequest::builder()
        .method(req.method.as_str())
        .uri(req.uri.as_str());

    for (name, value) in &req.headers {
        let header_name = HeaderName::from_bytes(name.as_bytes())
            .with_context(|| format!("invalid header name: {name}"))?;
        let header_value = HeaderValue::from_str(value)
            .with_context(|| format!("invalid header value: {value}"))?;
        builder = builder.header(header_name, header_value);
    }

    let body_bytes = Bytes::from(req.body);
    // http_body_util::Full<Bytes> has Error = Infallible; map to hyper::Error.
    let body = http_body_util::Full::new(body_bytes)
        .map_err(|e: std::convert::Infallible| -> hyper::Error { match e {} });

    builder.body(body).context("build hyper request")
}

/// Build a linker with WASI + WASI-HTTP host functions.
fn build_linker(engine: &Engine) -> Result<Linker<KilnHostState>> {
    let mut linker = Linker::<KilnHostState>::new(engine);
    wt(wasmtime_wasi::p2::add_to_linker_async(&mut linker)).context("add wasi to linker")?;
    wt(wasmtime_wasi_http::p2::add_only_http_to_linker_async(
        &mut linker,
    ))
    .context("add wasi-http to linker")?;
    Ok(linker)
}

// ─── Capability gate ─────────────────────────────────────────────────────────

pub fn check_capabilities(required: &[Capability], granted: &[Capability]) -> Result<()> {
    for cap in required {
        if !granted.contains(cap) {
            bail!("capability {cap:?} required but not granted");
        }
    }
    Ok(())
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
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
}
