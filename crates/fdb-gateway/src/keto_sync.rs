//! KetoSyncTask — periodically reads `flint_meta.keto_tuples` and makes them
//! available for Keto relation checks.
//!
//! # Security invariants
//!
//! - Runs as a **privileged Postgres role** (service account, not `authenticated`).
//!   Never sets `SET LOCAL ROLE authenticated`. The pool used here MUST be
//!   configured with a superuser or service-role credential.
//! - `keto_subject` values are PII — MUST NOT appear in any tracing span or log.
//! - If the database is unavailable, the last-known tuple set is held in memory
//!   until the next successful poll. The task NEVER clears the cache on error
//!   (fail-closed: stale data > empty data).
//!
//! # OQ-Iggy (open question)
//!
//! The authoritative source for relation tuples is FRF's Iggy `keto_changes` topic.
//! Until an Iggy Rust client is integrated (blocked on FRF team delivery of the
//! keto_changes schema), this task polls `flint_meta.keto_tuples` directly.
//! That table is populated by a separate Iggy consumer (FRF-side). The poll
//! interval defaults to 30 s; set `KETO_SYNC_INTERVAL_SECS` to override.
#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use fdb_ports::KetoCheck;
use sqlx::PgPool;
use tokio::sync::RwLock;
use tracing::instrument;

/// An in-memory view of `flint_meta.keto_tuples` populated by background polling.
///
/// `KetoCache` is shared via `Arc<RwLock<_>>` between the sync task and the
/// subscription handler. All reads are lock-free (read guard held briefly).
///
/// # Security
///
/// Cache entries are `(namespace, object, relation, subject_id)` tuples.
/// `subject_id` is PII — MUST NOT be logged, traced, or included in error messages.
#[derive(Debug, Default, Clone)]
pub struct KetoCacheEntry {
    // Fields read by the cache_check function (used once OQ-Iggy is resolved).
    #[allow(dead_code)]
    pub namespace: String,
    #[allow(dead_code)]
    pub object: String,
    #[allow(dead_code)]
    pub relation: String,
    #[allow(dead_code)]
    pub subject_id: String,
}

/// Shared Keto relation cache.
pub type KetoCache = Arc<RwLock<Vec<KetoCacheEntry>>>;

/// Configuration for the Keto sync task.
#[derive(Debug, Clone)]
pub struct KetoSyncConfig {
    /// Privileged Postgres pool — MUST NOT be the user RLS pool.
    /// The connection must use a service role with SELECT on `flint_meta.keto_tuples`.
    pub pool: Arc<PgPool>,
    /// How often to poll (defaults to 30 s; override with `KETO_SYNC_INTERVAL_SECS`).
    pub interval: Duration,
}

/// Background task that polls `flint_meta.keto_tuples` and refreshes `cache`.
///
/// # Guarantee
///
/// The task runs on its own Tokio task. It holds NO lock while sleeping —
/// the write lock is held only during the brief cache-replace operation.
///
/// If the poll fails, the existing cache is unchanged (fail-closed).
pub struct KetoSyncTask {
    config: KetoSyncConfig,
    cache: KetoCache,
}

impl KetoSyncTask {
    /// Create a new task and return both the task and its shared cache.
    pub fn new(config: KetoSyncConfig) -> (Self, KetoCache) {
        let cache = Arc::new(RwLock::new(Vec::new()));
        (
            Self {
                config,
                cache: Arc::clone(&cache),
            },
            cache,
        )
    }

    /// Spawn the background poll loop. Returns the `JoinHandle` so the caller
    /// can abort it on shutdown.
    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move { self.run().await })
    }

    async fn run(self) {
        tracing::info!(
            interval_secs = self.config.interval.as_secs(),
            "keto-sync starting"
        );

        // Initial poll before sleeping for the first interval.
        self.poll_once().await;

        loop {
            tokio::time::sleep(self.config.interval).await;
            self.poll_once().await;
        }
    }

    /// Poll `flint_meta.keto_tuples` and replace the cache on success.
    ///
    /// SECURITY: relation tuple values (`subject_id`) MUST NOT appear in spans.
    #[instrument(skip(self), fields(table = "flint_meta.keto_tuples"))]
    async fn poll_once(&self) {
        match fetch_keto_tuples(&self.config.pool).await {
            Ok(tuples) => {
                let count = tuples.len();
                *self.cache.write().await = tuples;
                tracing::debug!(count, "keto-sync refreshed cache");
            }
            Err(e) => {
                // Log error without any tuple values (fail-closed: keep stale cache).
                tracing::warn!(error = %e, "keto-sync poll failed; retaining stale cache");
            }
        }
    }
}

/// Row type returned by the privileged `flint_meta.keto_tuples` query.
/// `subject_id` is PII — never log it.
#[derive(sqlx::FromRow)]
struct KetoTupleRow {
    namespace: String,
    object: String,
    relation: String,
    subject_id: String,
}

/// Query `flint_meta.keto_tuples` using the privileged pool (no SET LOCAL ROLE).
///
/// This function deliberately does NOT set any RLS GUCs — the caller is responsible
/// for providing a pool that connects as a privileged role.
async fn fetch_keto_tuples(pool: &PgPool) -> Result<Vec<KetoCacheEntry>, sqlx::Error> {
    let rows: Vec<KetoTupleRow> = sqlx::query_as(
        "SELECT namespace, object, relation, subject_id FROM flint_meta.keto_tuples",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| KetoCacheEntry {
            namespace: r.namespace,
            object: r.object,
            relation: r.relation,
            subject_id: r.subject_id,
        })
        .collect())
}

/// Check whether `subject_id` has `relation` on `object` in `namespace`
/// using the in-memory Keto cache.
///
/// SECURITY: `subject_id` is PII — MUST NOT appear in the return value or logs.
/// Returns `true` only when a matching tuple exists; false otherwise (fail-closed).
///
/// Called once OQ-Iggy resolves and FabricChangeSource integrates this cache.
#[allow(dead_code)]
pub async fn cache_check(
    cache: &KetoCache,
    namespace: &str,
    object: &str,
    relation: &str,
    subject_id: &str,
) -> bool {
    let guard = cache.read().await;
    guard.iter().any(|entry| {
        entry.namespace == namespace
            && entry.object == object
            && entry.relation == relation
            && entry.subject_id == subject_id
    })
}

/// Default sync interval when `KETO_SYNC_INTERVAL_SECS` is unset or unparseable.
const DEFAULT_SYNC_INTERVAL_SECS: u64 = 30;

/// Resolve the sync interval from a raw `KETO_SYNC_INTERVAL_SECS` value.
///
/// Pure: takes the (optional) env value as an argument rather than reading the
/// process environment, so it is unit-testable in isolation without mutating
/// process-global state (which would race under parallel test execution).
/// A missing or non-numeric value falls back to [`DEFAULT_SYNC_INTERVAL_SECS`].
fn resolve_interval(raw: Option<&str>) -> Duration {
    let secs = raw
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_SYNC_INTERVAL_SECS);
    Duration::from_secs(secs)
}

/// Build `KetoSyncConfig` from environment variables and the provided privileged pool.
///
/// Reads `KETO_SYNC_INTERVAL_SECS` (default: 30) — the only env access; the parse
/// logic lives in the pure [`resolve_interval`].
pub fn keto_sync_config_from_env(pool: Arc<PgPool>) -> KetoSyncConfig {
    let raw = std::env::var("KETO_SYNC_INTERVAL_SECS").ok();
    KetoSyncConfig {
        pool,
        interval: resolve_interval(raw.as_deref()),
    }
}

/// Adapter that implements [`fdb_ports::KetoCheck`] over the shared [`KetoCache`].
///
/// This is the composition-time bridge between the gateway's background-synced
/// cache and the application layer's `Arc<dyn KetoCheck>` injection. It never
/// logs `subject` values (PII).
pub struct KetoCacheAdapter {
    cache: KetoCache,
}

impl KetoCacheAdapter {
    pub fn new(cache: KetoCache) -> Self {
        Self { cache }
    }
}

#[async_trait]
impl KetoCheck for KetoCacheAdapter {
    async fn check(&self, namespace: &str, object: &str, relation: &str, subject: &str) -> bool {
        cache_check(&self.cache, namespace, object, relation, subject).await
    }
}

// ─── unit tests ──────────────────────────────────────────────────────────────
//
// Integration tests (which require a real Postgres connection) are out of scope
// for the stub phase. The unit tests below cover the cache-check logic, the
// config-from-env reader, and the task construction without touching the database.

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cache_check_returns_true_on_matching_tuple() {
        let cache: KetoCache = Arc::new(RwLock::new(vec![KetoCacheEntry {
            namespace: "entities".into(),
            object: "orders".into(),
            relation: "view".into(),
            subject_id: "user-abc".into(),
        }]));

        assert!(
            cache_check(&cache, "entities", "orders", "view", "user-abc").await,
            "should find a matching tuple"
        );
    }

    #[tokio::test]
    async fn cache_check_returns_false_when_no_match() {
        let cache: KetoCache = Arc::new(RwLock::new(vec![KetoCacheEntry {
            namespace: "entities".into(),
            object: "orders".into(),
            relation: "view".into(),
            subject_id: "user-abc".into(),
        }]));

        // Wrong relation
        assert!(
            !cache_check(&cache, "entities", "orders", "write", "user-abc").await,
            "wrong relation should not match"
        );
        // Wrong subject
        assert!(
            !cache_check(&cache, "entities", "orders", "view", "user-xyz").await,
            "wrong subject should not match"
        );
    }

    #[tokio::test]
    async fn cache_check_returns_false_on_empty_cache() {
        let cache: KetoCache = Arc::new(RwLock::new(vec![]));
        assert!(
            !cache_check(&cache, "entities", "orders", "view", "user-abc").await,
            "empty cache should never allow"
        );
    }

    // These exercise the pure `resolve_interval` with literal inputs — no
    // process-env mutation, so they are deterministic under parallel execution
    // (the previous versions raced on the shared KETO_SYNC_INTERVAL_SECS var).

    #[test]
    fn resolve_interval_defaults_when_absent() {
        assert_eq!(resolve_interval(None), Duration::from_secs(30));
    }

    #[test]
    fn resolve_interval_reads_numeric_value() {
        assert_eq!(resolve_interval(Some("60")), Duration::from_secs(60));
        assert_eq!(resolve_interval(Some("1")), Duration::from_secs(1));
    }

    #[test]
    fn resolve_interval_falls_back_on_non_numeric() {
        assert_eq!(resolve_interval(Some("bad_value")), Duration::from_secs(30));
        assert_eq!(resolve_interval(Some("")), Duration::from_secs(30));
        assert_eq!(resolve_interval(Some("-5")), Duration::from_secs(30));
    }
}
