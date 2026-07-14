//! Request/response types exchanged between `EdgeRuntime` and its callers.

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
