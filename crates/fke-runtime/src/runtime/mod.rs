//! The Kiln data-plane runtime: `EdgeRuntime`, its Wasmtime host state, and
//! the per-invocation Cedar/capability gates.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use fke_domain::{Capability, ContentId};
use forge_identity::RlsContext;
use forge_policy::{Decision, Pep};
use http_body_util::BodyExt;
use wasmtime::component::{Component, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};
use wasmtime_wasi_http::p2::bindings::http::types::Scheme;
use wasmtime_wasi_http::p2::bindings::ProxyPre;
use wasmtime_wasi_http::p2::WasiHttpCtxView;
use wasmtime_wasi_http::p2::WasiHttpView;
use wasmtime_wasi_http::WasiHttpCtx;

use crate::capability::{capability_request, check_capabilities};
use crate::helpers::{build_linker, is_epoch_trap, kiln_request_to_hyper, wt};
use crate::types::{KilnHandleOutcome, KilnRequest, KilnResponse};

/// Default fuel grant per invocation (~10 M instructions).
const DEFAULT_FUEL: u64 = 10_000_000;

// ─── Host state ──────────────────────────────────────────────────────────────

pub(crate) struct KilnHostState {
    wasi_ctx: WasiCtx,
    table: ResourceTable,
    http_ctx: WasiHttpCtx,
    http_hooks: [(); 0], // zero-size default WasiHttpHooks impl
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
    /// Build a new, empty `EdgeRuntime` with no components loaded and no
    /// [`Pep`] attached (Cedar checks are skipped until [`Self::with_pep`] is
    /// called).
    ///
    /// Configures the Wasmtime `Engine` for the Component Model with fuel
    /// metering and epoch interruption enabled (see module docs), and — unless
    /// `KILN_EPOCH_INTERVAL_MS` is set to `0` — spawns a background task that
    /// increments the engine's epoch every `KILN_EPOCH_INTERVAL_MS`
    /// milliseconds (default 10 ms) so in-flight invocations can be trapped
    /// once they exceed their epoch deadline.
    ///
    /// # Errors
    ///
    /// Returns an error if `wasmtime::Engine::new` fails to initialize the
    /// engine (e.g. an unsupported target or invalid `Config`).
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

    /// Override the fuel budget granted to each invocation (default
    /// `DEFAULT_FUEL`, ~10 million instructions). A component that exhausts
    /// its fuel before returning is trapped by Wasmtime, bounding the cost of
    /// a runaway or malicious component.
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
    ///
    /// # Errors
    ///
    /// Returns an error if `wasm` is not a valid WASM component binary
    /// (`Component::from_binary`), if the WASI/WASI-HTTP linker cannot be
    /// built (`build_linker`), or if instantiation pre-computation fails
    /// (`Linker::instantiate_pre` or `ProxyPre::new` — typically because the
    /// component does not target the expected `wasi:http` proxy world).
    ///
    /// # Panics
    ///
    /// Panics if the internal component cache mutex is poisoned (a prior
    /// holder of the lock panicked while holding it).
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
    ///
    /// # Panics
    ///
    /// Panics if the internal component cache mutex is poisoned (a prior
    /// holder of the lock panicked while holding it).
    pub fn is_loaded(&self, id: &ContentId) -> bool {
        self.cache.lock().expect("cache lock").contains_key(id)
    }

    /// Dispatch an HTTP-style request to the loaded component.
    ///
    /// `caller = None` → Cedar gate skipped (BGW / system-level).
    /// Records `kiln_fuel_consumed_total` and `kiln_epoch_traps_total`.
    ///
    /// # Errors
    ///
    /// Returns an error under the same conditions as
    /// [`Self::handle_with_telemetry`]; see that method for the full list.
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
    ///
    /// `requested` is the component's declared `capabilities` (from its
    /// manifest) — what it asks to use, not what it is authorized for. The
    /// authorized set is computed here, per capability, via Cedar (or —
    /// matching the existing `caller = None` Cedar-skip convention for
    /// system-level/BGW invocations, and the no-`Pep`-attached case used by
    /// most unit tests — trusted wholesale).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - a [`Pep`] is attached and `caller` is `Some`, and Cedar denies
    ///   `kiln:invoke` for the caller;
    /// - any capability in `requested` is not present in the Cedar-authorized
    ///   `granted` set (see [`check_capabilities`](crate::check_capabilities));
    /// - no component is cached under `id` (it was never loaded, or was
    ///   evicted);
    /// - the per-request `Store` cannot be granted `fuel_per_call` fuel
    ///   (`Store::set_fuel`);
    /// - the request cannot be converted into a `hyper::Request`
    ///   (`kiln_request_to_hyper` — invalid header name/value);
    /// - the WASI-HTTP incoming request or response outparam cannot be
    ///   constructed;
    /// - the component fails to instantiate (`ProxyPre::instantiate_async`);
    /// - the invocation task panics, the handler itself errors, the component
    ///   never sets a response outparam, or it returns an HTTP error;
    /// - the response body cannot be collected into bytes.
    pub async fn handle_with_telemetry(
        &self,
        id: &ContentId,
        requested: &[Capability],
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
        // Real requested-vs-granted comparison (not the same value twice):
        // `granted` is computed independently, per capability, via Cedar.
        let granted = self.granted_capabilities(requested, caller).await;
        check_capabilities(requested, &granted)?;

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

    /// Compute the capabilities `caller` is actually authorized to use, out
    /// of `requested` (the component's manifest-declared capabilities).
    ///
    /// Matches the existing Cedar-skip convention: with no `Pep` attached, or
    /// `caller = None` (system-level/BGW invocation — already vetted by the
    /// `kiln:invoke` gate above, or intentionally ungated), every requested
    /// capability is trusted wholesale. Otherwise each capability is checked
    /// independently against Cedar, so `requested` and `granted` are never
    /// the same value by construction.
    async fn granted_capabilities(
        &self,
        requested: &[Capability],
        caller: Option<&RlsContext>,
    ) -> Vec<Capability> {
        let (Some(pep), Some(who)) = (&self.pep, caller) else {
            return requested.to_vec();
        };
        let mut granted = Vec::with_capacity(requested.len());
        for cap in requested {
            let decision = pep.check(who, &capability_request(cap)).await;
            if decision == Decision::Allow {
                granted.push(cap.clone());
            }
        }
        granted
    }
}

impl Default for EdgeRuntime {
    /// Build a runtime with default settings.
    ///
    /// # Panics
    ///
    /// Panics if [`EdgeRuntime::new`] fails (i.e. if `wasmtime::Engine`
    /// initialization fails). Prefer [`EdgeRuntime::new`] directly in
    /// contexts where engine construction failure must be handled instead of
    /// panicking.
    fn default() -> Self {
        Self::new().expect("EdgeRuntime::default")
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
