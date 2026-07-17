//! Request/response types exchanged between `EdgeRuntime` and its callers.

/// An HTTP-style request dispatched to a loaded Kiln component.
///
/// This is a transport-agnostic stand-in for a `hyper::Request` — callers of
/// `EdgeRuntime::handle`/`handle_with_telemetry` build one of these from
/// whatever inbound transport they use (Axum, gRPC, etc.), and
/// `kiln_request_to_hyper` converts it into the `hyper::Request` the WASI-HTTP
/// `incoming-handler` binding expects.
#[derive(Debug, Clone)]
pub struct KilnRequest {
    /// HTTP method (e.g. `"GET"`, `"POST"`) as a string; converted to a real
    /// method type when building the `hyper::Request`.
    pub method: String,
    /// Request URI (path plus optional query string) as seen by the component.
    pub uri: String,
    /// Request headers, in the order they should appear on the wire.
    /// Kept as raw `(name, value)` string pairs so the caller does not need a
    /// `hyper`/`http` dependency to construct a `KilnRequest`.
    pub headers: Vec<(String, String)>,
    /// Raw request body bytes.
    pub body: Vec<u8>,
}

/// The HTTP-style response produced by a Kiln component invocation.
#[derive(Debug, Clone)]
pub struct KilnResponse {
    /// HTTP status code returned by the component's `incoming-handler`.
    pub status: u16,
    /// Raw response body bytes collected from the component's output stream.
    pub body: Vec<u8>,
}

/// Telemetry captured during a single Kiln invocation.
#[derive(Debug, Clone)]
pub struct KilnHandleOutcome {
    /// The HTTP-style response the component produced.
    pub response: KilnResponse,
    /// Wasmtime fuel units consumed by this invocation (`initial - remaining`,
    /// see `EdgeRuntime::handle_with_telemetry`). Reported via the
    /// `kiln_fuel_consumed_total` counter and returned here so callers can
    /// attribute cost per-invocation (e.g. for billing or anomaly detection).
    pub fuel_consumed: u64,
    /// `true` if the invocation was aborted by an epoch-interruption trap
    /// (the component ran past its epoch deadline — see
    /// `EdgeRuntime::handle_with_telemetry` and `is_epoch_trap`) rather than
    /// completing or failing for another reason.
    pub epoch_trap: bool,
}
