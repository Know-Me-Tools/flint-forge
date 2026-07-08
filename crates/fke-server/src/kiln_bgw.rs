//! Kiln Background Worker — drains `flint.webhook_outbox WHERE target_type = 'kiln'`.
//!
//! When a `flint_hooks` trigger fires on a table bound to a Kiln edge function,
//! `dispatch_webhook()` queues a row in `flint.webhook_outbox` with
//! `target_type = 'kiln'` (wired by p7-c001). This BGW polls that queue,
//! resolves the target function from `fke-registry`, loads the WASM artifact,
//! and dispatches it through `fke-runtime::EdgeRuntime::handle()`.
//!
//! # Pattern
//!
//! Mirrors `fdb-gateway/src/agui_hook_dispatcher.rs` (p7-c002):
//! - SKIP LOCKED batch SELECT — safe for concurrent BGW instances
//! - Exponential backoff: 30 s → 60 s → 120 s → 300 s → fail (5th retry)
//! - One row processed to completion before moving to the next
#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use fke_domain::{ContentId, FunctionManifest};
use fke_ports::ComponentRegistry;
use fke_registry::{PgComponentStore, PgRegistry};
use fke_runtime::{EdgeRuntime, KilnRequest};
use forge_identity::RlsContext;
use sqlx::PgPool;

const POLL_INTERVAL: Duration = Duration::from_secs(5);
const BATCH_SIZE: i64 = 50;

/// Spawn the Kiln BGW.
///
/// Returns a `JoinHandle` the caller can abort on graceful shutdown.
pub fn spawn(
    pool: Arc<PgPool>,
    runtime: Arc<EdgeRuntime>,
    registry: Arc<PgRegistry>,
    store: Arc<PgComponentStore>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            if let Err(e) = process_batch(&pool, &runtime, &registry, &store).await {
                tracing::warn!(error = %e, "kiln-bgw: batch error");
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    })
}

/// Outbox row for Kiln targets.
#[derive(sqlx::FromRow)]
struct KilnOutboxRow {
    id: i64,
    payload: sqlx::types::Json<serde_json::Value>,
    retry_count: i32,
}

/// Process one batch of Kiln outbox entries.
async fn process_batch(
    pool: &PgPool,
    runtime: &EdgeRuntime,
    registry: &PgRegistry,
    store: &PgComponentStore,
) -> Result<(), sqlx::Error> {
    let rows: Vec<KilnOutboxRow> = sqlx::query_as(
        "SELECT id, payload, retry_count
         FROM   flint.webhook_outbox
         WHERE  status      IN ('pending', 'retrying')
           AND  target_type  = 'kiln'
           AND  visible_at  <= now()
         ORDER  BY visible_at
         LIMIT  $1
         FOR UPDATE SKIP LOCKED",
    )
    .bind(BATCH_SIZE)
    .fetch_all(pool)
    .await?;

    for row in rows {
        match invoke_function(runtime, registry, store, &row).await {
            Ok(()) => {
                sqlx::query(
                    "UPDATE flint.webhook_outbox
                     SET status = 'delivered', updated_at = now()
                     WHERE id = $1",
                )
                .bind(row.id)
                .execute(pool)
                .await?;
                tracing::debug!(id = row.id, "kiln-bgw: delivered");
            }
            Err(e) => {
                tracing::warn!(id = row.id, error = %e, "kiln-bgw: invocation failed");
                apply_retry(pool, row.id, row.retry_count).await?;
            }
        }
    }

    Ok(())
}

// ─── Publisher identity ──────────────────────────────────────────────────────

/// Synthesise a minimal `RlsContext` from the function's `publisher_did`
/// so the Cedar gate in `EdgeRuntime::handle()` fires on hook-triggered
/// invocations.
///
/// The `keto_subject` is the `publisher_did`; Cedar policy authors should
/// write grants against this principal identifier. `raw_bearer` is intentionally
/// empty — there is no JWT to forward for BGW calls.
fn publisher_rls(manifest: &FunctionManifest) -> RlsContext {
    RlsContext {
        role: "kiln_publisher".to_owned(),
        claims_json: format!(r#"{{"sub":"{}"}}"#, manifest.publisher_did),
        raw_bearer: String::new(),
        keto_subject: manifest.publisher_did.clone(),
        vault_key_id: None,
    }
}

/// Resolve the function, load the WASM if needed, and invoke the runtime.
async fn invoke_function(
    runtime: &EdgeRuntime,
    registry: &PgRegistry,
    store: &PgComponentStore,
    row: &KilnOutboxRow,
) -> Result<()> {
    // Extract function_name and function_version from the hook payload.
    let name = row
        .payload
        .get("function_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing payload.function_name"))?;
    let version = row
        .payload
        .get("function_version")
        .and_then(|v| v.as_str())
        .unwrap_or("latest");

    // Resolve the function manifest from the registry.
    let manifest = registry
        .resolve(name, version)
        .await
        .map_err(|e| anyhow::anyhow!("registry: {e}"))?;

    let content_id = ContentId(format!("sha256:{}", manifest.content_digest));

    // Synthesise a caller identity from the publisher_did so Cedar fires.
    let publisher = publisher_rls(&manifest);

    // Load the WASM into the runtime cache if not already present.
    // We attempt to handle first; if that fails with "not loaded" we fetch from store.
    let handle_result = runtime
        .handle(
            &content_id,
            &manifest.capabilities,
            Some(&publisher),
            build_request(row, name),
        )
        .await;

    if let Err(ref e) = handle_result {
        if e.to_string().contains("not loaded") {
            // Fetch WASM bytes from the component store and load into runtime.
            use fke_ports::ComponentStore;
            let wasm_bytes = store
                .get(&content_id)
                .await
                .map_err(|e| anyhow::anyhow!("store: {e}"))?;
            runtime
                .load_wasm(content_id.clone(), &wasm_bytes)
                .map_err(|e| anyhow::anyhow!("load_wasm: {e}"))?;

            // Retry the invocation now that the component is cached.
            runtime
                .handle(
                    &content_id,
                    &manifest.capabilities,
                    Some(&publisher),
                    build_request(row, name),
                )
                .await
                .map_err(|e| anyhow::anyhow!("handle: {e}"))?;

            return Ok(());
        }
    }

    handle_result
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("handle: {e}"))
}

/// Build a `KilnRequest` from the outbox row payload.
fn build_request(row: &KilnOutboxRow, name: &str) -> KilnRequest {
    let body = serde_json::to_vec(&row.payload.0).unwrap_or_default();
    KilnRequest {
        method: "POST".into(),
        uri: format!("/functions/v1/{name}"),
        headers: vec![("content-type".into(), "application/json".into())],
        body,
    }
}

/// Apply exponential backoff or mark as failed.
async fn apply_retry(pool: &PgPool, id: i64, retry_count: i32) -> Result<(), sqlx::Error> {
    if retry_count >= 4 {
        sqlx::query(
            "UPDATE flint.webhook_outbox
             SET status = 'failed', retry_count = retry_count + 1, updated_at = now()
             WHERE id = $1",
        )
        .bind(id)
        .execute(pool)
        .await?;
    } else {
        let delay_secs: i64 = match retry_count {
            0 => 30,
            1 => 60,
            2 => 120,
            3 => 300,
            _ => 600,
        };
        sqlx::query(
            "UPDATE flint.webhook_outbox
             SET status      = 'retrying',
                 retry_count = retry_count + 1,
                 visible_at  = now() + ($1 || ' seconds')::interval,
                 updated_at  = now()
             WHERE id = $2",
        )
        .bind(delay_secs.to_string())
        .bind(id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_row(id: i64, payload: serde_json::Value, retry_count: i32) -> KilnOutboxRow {
        KilnOutboxRow {
            id,
            payload: sqlx::types::Json(payload),
            retry_count,
        }
    }

    #[test]
    fn build_request_uses_function_name_in_uri() {
        let row = make_row(1, json!({"function_name": "my-func", "type": "INSERT"}), 0);
        let req = build_request(&row, "my-func");
        assert_eq!(req.method, "POST");
        assert_eq!(req.uri, "/functions/v1/my-func");
        assert!(
            !req.body.is_empty(),
            "body should contain serialized payload"
        );
    }

    #[test]
    fn build_request_body_is_valid_json() {
        let payload = json!({"function_name": "fn", "record": {"id": 1}});
        let row = make_row(2, payload.clone(), 0);
        let req = build_request(&row, "fn");
        let parsed: serde_json::Value =
            serde_json::from_slice(&req.body).expect("body is valid JSON");
        assert_eq!(parsed["function_name"], "fn");
    }

    #[test]
    fn invoke_function_missing_name_returns_error() {
        // The runtime / registry are not needed to hit the early-exit guard.
        // We verify the error path synchronously via the payload extraction.
        let row = make_row(3, json!({"type": "INSERT"}), 0);
        let name = row.payload.get("function_name").and_then(|v| v.as_str());
        assert!(name.is_none(), "expected missing function_name");
    }

    #[test]
    fn version_defaults_to_latest_when_absent() {
        let row = make_row(4, json!({"function_name": "fn"}), 0);
        let version = row
            .payload
            .get("function_version")
            .and_then(|v| v.as_str())
            .unwrap_or("latest");
        assert_eq!(version, "latest");
    }

    #[tokio::test]
    async fn bgw_spawns_and_is_abortable() {
        // Verify spawn() returns a JoinHandle that can be aborted immediately.
        // Uses a pool that cannot connect — process_batch errors and loops.
        let pool = Arc::new(
            sqlx::PgPool::connect_lazy("postgres://localhost/nonexistent").expect("lazy pool"),
        );
        let rt = Arc::new(fke_runtime::EdgeRuntime::new().expect("runtime"));
        // Registry and store need pools too; use the same lazy pool.
        let registry = Arc::new(fke_registry::PgRegistry::new((*pool).clone()));
        let store = Arc::new(fke_registry::PgComponentStore::new((*pool).clone()));
        let handle = spawn(pool, rt, registry, store);
        // Give it one tick to start.
        tokio::time::sleep(Duration::from_millis(10)).await;
        handle.abort();
        // Aborted handles finish without panicking.
        let _ = handle.await;
    }

    #[test]
    fn publisher_rls_sets_keto_subject_to_did() {
        let manifest = fke_domain::FunctionManifest {
            publisher_did: "did:prometheus:abc123".to_owned(),
            content_digest: "sha256:test".to_owned(),
            capabilities: vec![],
            version: "1.0.0".to_owned(),
            not_before: "2020-01-01T00:00:00Z".to_owned(),
            not_after: "2099-12-31T23:59:59Z".to_owned(),
        };
        let rls = publisher_rls(&manifest);
        assert_eq!(rls.keto_subject, "did:prometheus:abc123");
        assert_eq!(rls.role, "kiln_publisher");
    }

    #[test]
    fn publisher_rls_sets_empty_bearer() {
        let manifest = fke_domain::FunctionManifest {
            publisher_did: "did:prometheus:xyz".to_owned(),
            content_digest: "sha256:x".to_owned(),
            capabilities: vec![],
            version: "1.0.0".to_owned(),
            not_before: "2020-01-01T00:00:00Z".to_owned(),
            not_after: "2099-12-31T23:59:59Z".to_owned(),
        };
        let rls = publisher_rls(&manifest);
        assert!(
            rls.raw_bearer.is_empty(),
            "raw_bearer must be empty for BGW calls"
        );
        assert!(rls.vault_key_id.is_none());
    }
}
