//! Flint Ember background worker.
//!
//! Registered via `shared_preload_libraries = 'ext_flint_llm'`, this worker
//! polls `llm.jobs`, batches embedding requests through flint-gate/UAR, and
//! writes the resulting vectors back to application tables.

use crate::gate_client::GateClient;
use crate::governor::{Governor, LimitReason};
use crate::jobs::JobRow;
use pgrx::bgworkers::{BackgroundWorker, BackgroundWorkerBuilder, SignalWakeFlags};
use pgrx::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Name used by `shared_preload_libraries` and `BackgroundWorkerBuilder`.
const WORKER_NAME: &str = "flint_llm_worker";
const BATCH_SIZE: i64 = 8;
const POLL_INTERVAL: Duration = Duration::from_secs(5);

/// Build a flint-gate client from extension configuration.
fn build_client() -> Option<GateClient> {
    let base_url = crate::gate_client::default_base_url();
    let token = match crate::credentials::resolve_service_token() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{WORKER_NAME}: failed to resolve service token: {e}");
            return None;
        }
    };
    match GateClient::new(base_url, token) {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!("{WORKER_NAME}: failed to build gate client: {e}");
            None
        }
    }
}

/// Extract the text payload from a job's `source` JSONB.
fn extract_text(job: &JobRow) -> String {
    job.source
        .as_ref()
        .and_then(|s| s.get("text").and_then(|v| v.as_str()).map(String::from))
        .unwrap_or_default()
}

/// Extract the rendered prompt from a job's `source` JSONB.
fn extract_prompt(job: &JobRow) -> String {
    job.source
        .as_ref()
        .and_then(|s| s.get("prompt").and_then(|v| v.as_str()).map(String::from))
        .unwrap_or_default()
}

/// Process a single job: rate-limit, call flint-gate, write back, mark status.
fn process_one(
    job: JobRow,
    rt: &tokio::runtime::Runtime,
    client: &GateClient,
    governor: &Mutex<Governor>,
) {
    match job.kind.as_str() {
        "embed" => process_embed(job, rt, client, governor),
        "summarize" => process_summarize(job, rt, client, governor),
        other => {
            let _ = crate::jobs::mark_failed(
                job.id,
                job.retry_count,
                false,
                &format!("unknown job kind: {other}"),
            );
        }
    }
}

fn schema_table_column(job: &JobRow) -> Option<(&str, &str, &str)> {
    let schema = job.schema_name.as_deref().unwrap_or("public");
    let table = job.table_name.as_deref()?;
    let column = job.target_column.as_deref()?;
    Some((schema, table, column))
}

fn process_embed(
    job: JobRow,
    rt: &tokio::runtime::Runtime,
    client: &GateClient,
    governor: &Mutex<Governor>,
) {
    let text = extract_text(&job);
    if text.is_empty() {
        let _ = crate::jobs::mark_failed(job.id, job.retry_count, false, "empty source text");
        return;
    }

    let token_count = Governor::estimate_tokens(&text);
    if let Err(reason) = acquire_with_log(
        &job,
        governor,
        token_count,
        job.model.as_deref().unwrap_or("default"),
    ) {
        if reason == LimitReason::Rpm {
            std::thread::sleep(Duration::from_secs(1));
        }
        return;
    }

    match rt.block_on(client.embed(&text, job.model.as_deref(), job.origin_jwt.as_deref())) {
        Ok(vector) => {
            let (schema, table, column) = match schema_table_column(&job) {
                Some(v) => v,
                None => {
                    let _ = crate::jobs::mark_failed(
                        job.id,
                        job.retry_count,
                        false,
                        "missing table_name or target_column",
                    );
                    return;
                }
            };
            if let Err(e) = crate::writeback::write_vector(
                schema,
                table,
                job.pk.as_ref(),
                column,
                &vector,
                job.dimensions,
            ) {
                eprintln!("{WORKER_NAME}: writeback failed for job {}: {e}", job.id);
                let retry = job.retry_count < 3;
                let _ = crate::jobs::mark_failed(
                    job.id,
                    job.retry_count,
                    retry,
                    &format!("writeback: {e}"),
                );
                return;
            }
            if let Err(e) = crate::jobs::mark_completed(job.id) {
                eprintln!(
                    "{WORKER_NAME}: failed to mark job {} completed: {e}",
                    job.id
                );
            }
        }
        Err(e) => {
            eprintln!("{WORKER_NAME}: embed failed for job {}: {e}", job.id);
            let retry = job.retry_count < 3;
            let _ = crate::jobs::mark_failed(job.id, job.retry_count, retry, &e.to_string());
        }
    }
}

fn process_summarize(
    job: JobRow,
    rt: &tokio::runtime::Runtime,
    client: &GateClient,
    governor: &Mutex<Governor>,
) {
    let prompt = extract_prompt(&job);
    if prompt.is_empty() {
        let _ = crate::jobs::mark_failed(job.id, job.retry_count, false, "empty prompt");
        return;
    }

    let token_count = Governor::estimate_tokens(&prompt);
    if let Err(reason) = acquire_with_log(
        &job,
        governor,
        token_count,
        job.model.as_deref().unwrap_or("default"),
    ) {
        if reason == LimitReason::Rpm {
            std::thread::sleep(Duration::from_secs(1));
        }
        return;
    }

    match rt.block_on(client.complete(
        &prompt,
        job.model.as_deref(),
        None,
        job.origin_jwt.as_deref(),
    )) {
        Ok(text) => {
            let (schema, table, column) = match schema_table_column(&job) {
                Some(v) => v,
                None => {
                    let _ = crate::jobs::mark_failed(
                        job.id,
                        job.retry_count,
                        false,
                        "missing table_name or target_column",
                    );
                    return;
                }
            };
            if let Err(e) =
                crate::writeback::write_text(schema, table, job.pk.as_ref(), column, &text)
            {
                eprintln!(
                    "{WORKER_NAME}: text writeback failed for job {}: {e}",
                    job.id
                );
                let retry = job.retry_count < 3;
                let _ = crate::jobs::mark_failed(
                    job.id,
                    job.retry_count,
                    retry,
                    &format!("writeback: {e}"),
                );
                return;
            }
            if let Err(e) = crate::jobs::mark_completed(job.id) {
                eprintln!(
                    "{WORKER_NAME}: failed to mark job {} completed: {e}",
                    job.id
                );
            }
        }
        Err(e) => {
            eprintln!("{WORKER_NAME}: summarize failed for job {}: {e}", job.id);
            let retry = job.retry_count < 3;
            let _ = crate::jobs::mark_failed(job.id, job.retry_count, retry, &e.to_string());
        }
    }
}

fn acquire_with_log(
    job: &JobRow,
    governor: &Mutex<Governor>,
    token_count: u64,
    model: &str,
) -> Result<(), LimitReason> {
    let mut guard = match governor.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.try_acquire(token_count, model).map_err(|reason| {
        eprintln!("{WORKER_NAME}: job {} rate limited: {reason:?}", job.id);
        // Back the job off a short while so we don't immediately re-dequeue it.
        let _ = crate::jobs::mark_failed(job.id, job.retry_count, true, "rate limited");
        reason
    })
}

/// Main worker loop. Runs until SIGTERM or until the latch is released.
#[unsafe(no_mangle)]
#[pg_guard]
pub extern "C-unwind" fn flint_llm_worker_main(_arg: pg_sys::Datum) {
    BackgroundWorker::attach_signal_handlers(SignalWakeFlags::SIGHUP | SignalWakeFlags::SIGTERM);
    BackgroundWorker::connect_worker_to_spi(Some("postgres"), None);

    let rt = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("{WORKER_NAME}: failed to create tokio runtime: {e}");
            return;
        }
    };

    let client = match build_client() {
        Some(c) => c,
        None => return,
    };

    let governor = Arc::new(Mutex::new(Governor::default()));

    while BackgroundWorker::wait_latch(Some(POLL_INTERVAL)) {
        let batch = match crate::jobs::dequeue_pending(BATCH_SIZE) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("{WORKER_NAME}: dequeue failed: {e}");
                std::thread::sleep(Duration::from_secs(1));
                continue;
            }
        };

        if batch.is_empty() {
            continue;
        }

        for job in batch {
            process_one(job, &rt, &client, &governor);
        }
    }
}

/// True if we are running inside a background worker process.
fn in_background_worker() -> bool {
    unsafe { !pg_sys::MyBgworkerEntry.is_null() }
}

/// Register the background worker when the extension library is loaded.
#[pg_guard]
pub extern "C-unwind" fn _PG_init() {
    crate::sync::register_sync_gucs();

    // The async background worker is opt-in for v1.0. It is only safe to
    // register from the postmaster at shared_preload_libraries load time.
    // When the library is re-loaded inside the worker itself, do not recurse.
    if !crate::sync::background_worker_enabled() || in_background_worker() {
        return;
    }

    BackgroundWorkerBuilder::new(WORKER_NAME)
        .set_function("flint_llm_worker_main")
        .set_library("flint_llm")
        .enable_spi_access()
        .load();
}
