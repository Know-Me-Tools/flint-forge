//! Small conversions and adapters shared by the runtime and compiler modules.

use anyhow::{Context, Result};
use http_body_util::BodyExt;
use hyper::body::Bytes;
use hyper::header::{HeaderName, HeaderValue};
use hyper::Request as HyperRequest;
use wasmtime::component::Linker;
use wasmtime::Engine;

use crate::runtime::KilnHostState;
use crate::types::KilnRequest;

// wasmtime 46 uses its own Error type that does not implement std::error::Error.
// This helper bridges wasmtime::Result<T> → anyhow::Result<T> so .context() works.
#[inline]
pub(crate) fn wt<T>(r: wasmtime::Result<T>) -> core::result::Result<T, anyhow::Error> {
    r.map_err(|e| anyhow::anyhow!("{e}"))
}

/// Detect whether a wasmtime error was caused by an epoch-interruption trap.
pub(crate) fn is_epoch_trap(e: &wasmtime::Error) -> bool {
    e.to_string().to_lowercase().contains("epoch")
}

/// Convert a `KilnRequest` into a `hyper::Request` compatible with
/// `WasiHttpView::new_incoming_request` (`Body<Data=Bytes, Error=hyper::Error>`).
pub(crate) fn kiln_request_to_hyper(
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
///
/// WASI preview2 + WASI-HTTP are the required baseline for `ProxyPre` (every
/// edge function targets `wit/flint/host/world.wit`'s `edge-function` world,
/// which is a `wasi:http` proxy) — they are not one of `fke_domain::Capability`'s
/// governed capabilities, and are not gated per-component the way `flint:host`'s
/// custom `db`/`llm`/`kv`/`identity`/`secrets` interfaces are (p16-c003 design
/// doc). The real per-invocation sandbox boundary for WASI itself is the
/// `WasiCtx` built fresh in `handle_with_telemetry` (`WasiCtxBuilder::new()
/// .inherit_stdio().build()`), which grants no filesystem preopens, network,
/// env vars, or args beyond stdio passthrough.
///
/// `flint:host`'s five custom interfaces are deliberately NOT wired here yet:
/// `db`/`llm`/`secrets` need live backing clients (a flint-gate DB proxy, a
/// UAR/LLM gateway client, a flint_vault client) this crate doesn't have
/// access to, and none of the five can be verified end-to-end without the
/// `cargo-component` toolchain (unavailable in this environment — matching
/// the pre-existing self-skip in this module's own WASM-execution gate
/// tests) to build even one real test component that imports them. A
/// component declaring any of these capabilities today fails to instantiate
/// with a "missing import" error — fail-closed, not fail-open, but also not
/// yet functional. Tracked as a follow-up (spawned during p16-c003).
pub(crate) fn build_linker(engine: &Engine) -> Result<Linker<KilnHostState>> {
    let mut linker = Linker::<KilnHostState>::new(engine);
    wt(wasmtime_wasi::p2::add_to_linker_async(&mut linker)).context("add wasi to linker")?;
    wt(wasmtime_wasi_http::p2::add_only_http_to_linker_async(
        &mut linker,
    ))
    .context("add wasi-http to linker")?;
    Ok(linker)
}
