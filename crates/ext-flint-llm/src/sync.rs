//! Synchronous LLM surface for Flint Ember.
//!
//! These functions are intended for explicit, read-only or low-volume use. They
//! run flint-gate requests on a dedicated OS thread with a hard wall-clock
//! timeout so a misbehaving upstream cannot wedge a Postgres backend.

use crate::error::{LlmError, Result};
use crate::gate_client::GateClient;
use pgrx::guc::{GucContext, GucFlags, GucRegistry, GucSetting};
use pgrx::prelude::*;
use pgrx::JsonB;
use std::time::Duration;

/// Maximum wall-clock time (milliseconds) for synchronous LLM/embed calls.
/// Can be changed per session/transaction with `SET llm.sync_timeout_ms = ...`.
static SYNC_TIMEOUT_MS: GucSetting<i32> = GucSetting::<i32>::new(30_000);

/// Register GUCs used by the sync surface. Called from `_PG_init`.
pub fn register_sync_gucs() {
    GucRegistry::define_int_guc(
        c"llm.sync_timeout_ms",
        c"Maximum milliseconds to wait for a synchronous LLM call.",
        c"Applies to llm.embed() and llm.complete(). A value <= 0 falls back to 1 ms.",
        &SYNC_TIMEOUT_MS,
        1,
        i32::MAX,
        GucContext::Userset,
        GucFlags::UNIT_MS,
    );
}

fn sync_timeout_ms() -> i32 {
    SYNC_TIMEOUT_MS.get().max(1)
}

/// Resolve the origin JWT from the current session, if present.
///
/// This mirrors how the async trigger captures attribution for Cedar policy
/// enforcement inside flint-gate/UAR.
fn current_origin_jwt() -> Option<String> {
    Spi::get_one::<String>("SELECT current_setting('request.jwt', true)")
        .ok()
        .flatten()
}

/// Build a single-threaded tokio runtime for the isolated sync thread.
fn build_runtime() -> Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .map_err(|e| LlmError::Config(format!("failed to build tokio runtime: {e}")))
}

/// Run a closure on a dedicated thread, returning its result or a timeout error.
fn run_with_timeout<T, F>(timeout_ms: i32, f: F) -> Result<T>
where
    F: FnOnce() -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });

    let timeout = Duration::from_millis(timeout_ms as u64);
    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err(LlmError::Timeout(timeout)),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Err(LlmError::Interrupted),
    }
}

/// Format an embedding vector as the pgvector text literal accepted by `::vector`.
fn format_vector(v: &[f32]) -> String {
    format!(
        "[{}]",
        v.iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(",")
    )
}

/// Internal synchronous embedding helper.
///
/// Returns a text vector literal (e.g. `[0.1,0.2,...]`). The public `llm.embed`
/// SQL wrapper casts this to `vector`.
#[pg_extern]
fn _embed_text(input: &str, model: Option<&str>) -> Result<String> {
    let input = input.to_string();
    let model = model.map(String::from);
    let origin_jwt = current_origin_jwt();
    let base_url = crate::gate_client::default_base_url();
    let token = crate::credentials::resolve_service_token()?;
    let timeout = sync_timeout_ms();

    run_with_timeout(timeout, move || {
        let rt = build_runtime()?;
        let client = GateClient::new(base_url, token)?;
        let vector = rt.block_on(client.embed(&input, model.as_deref(), origin_jwt.as_deref()))?;
        Ok(format_vector(&vector))
    })
}

/// Internal synchronous completion helper.
///
/// Returns the raw text content from the gateway. The public `llm.complete`
/// SQL wrapper forwards options and model defaults.
#[pg_extern]
fn _complete(prompt: &str, model: Option<&str>, options: Option<JsonB>) -> Result<String> {
    let prompt = prompt.to_string();
    let model = model.map(String::from);
    let options = options.map(|j| j.0);
    let origin_jwt = current_origin_jwt();
    let base_url = crate::gate_client::default_base_url();
    let token = crate::credentials::resolve_service_token()?;
    let timeout = sync_timeout_ms();

    run_with_timeout(timeout, move || {
        let rt = build_runtime()?;
        let client = GateClient::new(base_url, token)?;
        let content = rt.block_on(client.complete(
            &prompt,
            model.as_deref(),
            options.as_ref(),
            origin_jwt.as_deref(),
        ))?;
        Ok(content)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_literal_formatting() {
        assert_eq!(format_vector(&[0.1, 0.2, 0.3]), "[0.1,0.2,0.3]");
    }

    #[test]
    fn timeout_helper_returns_value() {
        let result = run_with_timeout(1000, || Ok(42));
        assert_eq!(result.unwrap(), 42);
    }
}
