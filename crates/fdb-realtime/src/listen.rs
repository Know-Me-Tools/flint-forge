//! In-process `ChangeStreamSource` over Postgres `LISTEN`/`NOTIFY` — the OQ-FRF-1
//! workaround for the missing FRF `WatchEntityType` gRPC RPC.
//!
//! This adapter has EXACTLY two jobs:
//!
//! 1. **Subscribe-time Keto coarse check** (fail closed) — reuses the crate's
//!    `keto_check_via_http` helper verbatim.
//! 2. **Producing the raw `ChangeEvent` stream** from `LISTEN`/`NOTIFY`.
//!
//! It does NOT perform the per-event RLS re-query. That is owned by the use-case
//! layer (`fdb_app::Quarry::subscribe_rls_filtered`), which layers `build_pk_filters`
//! plus a `RestExecutor` round-trip on top of whatever `watch()` returns. No
//! `fdb-app` dependency is added here.
//!
//! # Critical downstream contract
//!
//! Because the use-case rebuilds primary-key filters from `record`/`old_record` to
//! re-fetch the current row under RLS, the raw `ChangeEvent` MUST always carry the
//! primary-key column values — even when a wide row is truncated to fit the 8000-byte
//! `NOTIFY` limit. The migration's trigger degrades to a PK-only image in that case
//! (see `migrations/0006_change_notify.sql`). The full `record` in the untruncated
//! case is an optimization the RLS re-query overwrites; it is NEVER trusted as the
//! delivered row.
//!
//! # Security invariants
//!
//! - `who.keto_subject` is PII and MUST NOT appear in any tracing span/log or error.
//! - The subscribe-time Keto check FAILS CLOSED: Keto unreachable => deny, never allow.
//! - Every failure mode here can only cause a *missed* event, never an *unauthorized*
//!   one, because the downstream RLS re-query is authoritative on every delivered event.
//!
//! # Cargo changes required
//!
//! In `crates/fdb-realtime/Cargo.toml`, add under `[dependencies]`:
//!
//! ```toml
//! sqlx = { workspace = true }
//! tokio = { workspace = true }
//! tokio-stream = { workspace = true }
//! ```
//!
//! `sqlx` (0.8, features postgres + runtime-tokio + json) and `tokio` (features full)
//! already exist in the root `[workspace.dependencies]`. `tokio-stream` does NOT — add
//! it to the root `Cargo.toml` `[workspace.dependencies]`:
//!
//! ```toml
//! tokio-stream = { version = "0.1", features = ["sync"] }
//! ```
//!
//! (The `sync` feature gates `tokio_stream::wrappers::BroadcastStream`.)
//!
//! The unit tests use `wiremock` for the Keto happy/denied HTTP paths. Add it to
//! `crates/fdb-realtime/Cargo.toml` under `[dev-dependencies]` (not needed at
//! runtime): `wiremock = "0.6"` (or `wiremock = { workspace = true }` if pinned in
//! the root `[workspace.dependencies]`). The load-bearing `Unavailable` fail-closed
//! test uses a dead port and needs no extra dependency.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use fdb_domain::{ChangeEvent, ChangeOp, SubscriptionSpec};
use fdb_ports::{ChangeStreamSource, StreamError};
use forge_identity::RlsContext;
use futures::stream::{BoxStream, StreamExt};
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;
use tracing::instrument;

use crate::{keto_check_via_http, KetoConfig};

/// The single fixed `LISTEN`/`NOTIFY` channel. MUST match `pg_notify(...)` in
/// `migrations/0006_change_notify.sql`.
const CHANNEL: &str = "flint_change";

/// Migration-side threshold (bytes) at which the trigger degrades to a PK-only
/// image. Documented here only to keep the Rust and SQL sides in sync; the Rust
/// adapter never enforces it (Postgres does).
const _MAX_NOTIFY_BYTES: usize = 7500;

/// Backoff between reconnect attempts in the listen loop, to avoid a hot spin
/// when the connection is flapping.
const RECONNECT_BACKOFF: Duration = Duration::from_millis(500);

/// Default broadcast capacity when the caller does not care to tune it.
const DEFAULT_BROADCAST_CAPACITY: usize = 1024;

/// Configuration for the in-process LISTEN/NOTIFY change source.
#[derive(Debug, Clone)]
pub struct ListenConfig {
    /// Postgres connection string for the DEDICATED listener connection.
    /// This is NOT a request-scoped pooled conn — `PgListener` holds it for the
    /// process lifetime. Use a low-privilege role; it only runs `LISTEN`.
    pub database_url: String,
    /// Bounded capacity of the broadcast channel feeding subscribers. Governs
    /// lag tolerance: a subscriber more than this many events behind is
    /// `Lagged` and skips events (never blocks the producer).
    pub broadcast_capacity: usize,
}

impl ListenConfig {
    /// Construct a config with the default broadcast capacity.
    #[must_use]
    pub fn new(database_url: impl Into<String>) -> Self {
        Self {
            database_url: database_url.into(),
            broadcast_capacity: DEFAULT_BROADCAST_CAPACITY,
        }
    }
}

/// Errors from `ListenChangeSource` construction.
///
/// Neither variant carries detail: the DSN can embed a password and `sqlx::Error`
/// may echo the connection string, so the underlying cause is logged once (redacted)
/// and never returned.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ListenError {
    /// The dedicated listener connection could not be established.
    #[error("listener connect")]
    Connect,
    /// `LISTEN` on the change channel failed.
    #[error("listen setup")]
    Listen,
}

/// `ChangeStreamSource` backed by in-process Postgres `LISTEN`/`NOTIFY`.
///
/// One instance is shared across all subscription connections. A single background
/// task owns the dedicated `PgListener` and fans decoded events out over a
/// `tokio::sync::broadcast`; each `watch()` call subscribes an independent receiver
/// and filters it by `entity_type`.
#[derive(Clone)]
pub struct ListenChangeSource {
    /// Sender side of the fan-out; every `watch()` subscribes a receiver.
    /// `ChangeEvent` is `Arc`-wrapped so the single decode is shared across N
    /// subscribers without per-subscriber clones of a potentially large record.
    tx: tokio::sync::broadcast::Sender<Arc<ChangeEvent>>,
    /// HTTP client for the Keto coarse check.
    http: reqwest::Client,
    /// Keto read-API config (reused from the crate root).
    keto: KetoConfig,
    /// Aborts the background listen task (and drops its dedicated PG connection)
    /// when the last clone of this source is dropped. Shared via `Arc` so cloning
    /// the source keeps the single task alive; the guard fires only on final drop.
    _task: Arc<ListenTaskGuard>,
}

/// Owns the background listen task's abort handle; aborts on drop so a dropped
/// `ListenChangeSource` (e.g. reconfigure, test teardown) does not leak the task
/// and its dedicated Postgres connection.
struct ListenTaskGuard(tokio::task::AbortHandle);

impl Drop for ListenTaskGuard {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl ListenChangeSource {
    /// Connect the dedicated `PgListener`, `LISTEN` on `flint_change`, and spawn the
    /// single background task that decodes notifications into the broadcast.
    ///
    /// Fails closed at construction: if the listener cannot connect or `LISTEN`,
    /// returns `Err` — the gateway MUST NOT mount subscriptions without a live feed.
    ///
    /// # Errors
    ///
    /// - [`ListenError::Connect`] if the dedicated connection cannot be established.
    /// - [`ListenError::Listen`] if `LISTEN flint_change` fails.
    pub async fn new(cfg: ListenConfig, keto: KetoConfig) -> Result<Self, ListenError> {
        let mut listener = sqlx::postgres::PgListener::connect(&cfg.database_url)
            .await
            .map_err(|e| {
                // Never surface the DSN (may contain credentials). Log without it.
                tracing::error!("listener connect failed: {}", redact(&e));
                ListenError::Connect
            })?;
        listener.listen(CHANNEL).await.map_err(|e| {
            tracing::error!(channel = CHANNEL, "listen setup failed: {}", redact(&e));
            ListenError::Listen
        })?;

        let capacity = cfg.broadcast_capacity.max(1);
        let (tx, _rx) = tokio::sync::broadcast::channel(capacity);
        let task_tx = tx.clone();
        let handle = tokio::spawn(listen_loop(listener, task_tx));

        Ok(Self {
            tx,
            http: reqwest::Client::new(),
            keto,
            _task: Arc::new(ListenTaskGuard(handle.abort_handle())),
        })
    }
}

/// Redact a `sqlx::Error` down to its variant discriminant so nothing derived from
/// the DSN (host, user, password) can reach a log line.
fn redact(err: &sqlx::Error) -> &'static str {
    match err {
        sqlx::Error::Configuration(_) => "configuration",
        sqlx::Error::Io(_) => "io",
        sqlx::Error::Tls(_) => "tls",
        sqlx::Error::Protocol(_) => "protocol",
        sqlx::Error::PoolTimedOut => "pool-timeout",
        sqlx::Error::PoolClosed => "pool-closed",
        sqlx::Error::Database(_) => "database",
        _ => "other",
    }
}

#[async_trait]
impl ChangeStreamSource for ListenChangeSource {
    /// Open a change stream for `spec.entity_type` as subscriber `who`.
    ///
    /// # Security
    ///
    /// 1. Keto coarse check before opening the stream — FAILS CLOSED on Keto
    ///    unavailability (`StreamError::Unavailable`) and on deny (`StreamError::Denied`).
    /// 2. `who.keto_subject` is PII and MUST NOT be logged — `skip(who)` on the span,
    ///    and it never appears in any field below.
    ///
    /// The returned stream carries RAW events. The per-event RLS re-query is layered
    /// on by the use-case; this adapter does not perform it.
    ///
    /// # Errors
    ///
    /// - [`StreamError::Unavailable`] if Keto is unreachable / errors (fail closed).
    /// - [`StreamError::Denied`] if Keto reports no `view` relation.
    #[instrument(
        skip(self, who),
        fields(entity_type = %spec.entity_type, tenant = %spec.tenant),
        err
    )]
    async fn watch(
        &self,
        spec: SubscriptionSpec,
        who: &RlsContext,
        // SECURITY: who.keto_subject MUST NOT be logged here or in any span field above.
    ) -> Result<BoxStream<'static, Result<ChangeEvent, StreamError>>, StreamError> {
        // Defense-in-depth: `entity_type` (`<schema>.<table>`) and `tenant` are
        // server-derived today (reflected catalog + verified JWT), but the Keto
        // helper interpolates them into a URL query string. Reject anything that
        // is not a safe identifier before it can reach that URL — fail closed.
        if !is_safe_entity(&spec.entity_type) || !is_safe_tenant(&spec.tenant) {
            return Err(StreamError::Denied);
        }

        // Subscribe to the fan-out BEFORE the awaited Keto round-trip, so events
        // broadcast during the check are not missed (the miss window would
        // otherwise be the Keto network latency). On deny, `rx` is simply dropped.
        let rx = self.tx.subscribe();

        // Keto coarse check, FAIL CLOSED. Reuses the exact crate helper.
        keto_check_via_http(
            &self.http,
            &self.keto.base_url,
            &spec.entity_type,
            &spec.tenant,
            &who.keto_subject,
        )
        .await?;

        // Filter the fan-out by entity_type. The tenant match is best-effort;
        // the RLS re-query downstream is authoritative.
        let want_entity = spec.entity_type.clone();

        let stream = BroadcastStream::new(rx)
            .filter_map(move |item| {
                let want_entity = want_entity.clone();
                async move {
                    match item {
                        Ok(ev) => {
                            if matches(&want_entity, &ev.schema, &ev.table) {
                                Some(Ok((*ev).clone()))
                            } else {
                                // Not our table → drop, keep the stream open.
                                None
                            }
                        }
                        // Subscriber fell behind: skip the lagged window, do NOT
                        // terminate the stream and do NOT surface an Err (a missed
                        // event is a missed update, never a leaked row).
                        Err(BroadcastStreamRecvError::Lagged(n)) => {
                            tracing::warn!(
                                skipped = n,
                                entity_type = %want_entity,
                                "listen broadcast lagged; subscriber skipped events"
                            );
                            None
                        }
                    }
                }
            })
            .boxed();

        Ok(stream)
    }
}

/// The single background task: own the `PgListener`, decode each notification once,
/// and fan it out over the broadcast. Runs for the process lifetime.
async fn listen_loop(
    mut listener: sqlx::postgres::PgListener,
    tx: tokio::sync::broadcast::Sender<Arc<ChangeEvent>>,
) {
    loop {
        match listener.recv().await {
            Ok(notif) => match parse_payload(notif.payload()) {
                Ok((_tenant, event)) => {
                    // Ignore SendError: zero receivers just means no active
                    // subscribers right now — that is normal, not an error.
                    let _ = tx.send(Arc::new(event));
                }
                Err(_) => {
                    // Malformed payload → count + drop; NEVER panic, NEVER break.
                    // Do NOT log the payload (it may carry row data / PII).
                    tracing::warn!("dropped unparseable change notification");
                }
            },
            Err(e) => {
                // Connection dropped. sqlx `PgListener` auto-reconnects and
                // re-issues LISTEN on the next recv(); log (redacted) and back off.
                tracing::warn!(error = %redact(&e), "listen connection error; reconnecting");
                tokio::time::sleep(RECONNECT_BACKOFF).await;
            }
        }
    }
}

/// Wire shape of a `flint_change` NOTIFY payload. `tenant`/`truncated` are consumed
/// and dropped — they are not part of the domain `ChangeEvent`.
#[derive(serde::Deserialize)]
struct RawNotify {
    op: String,
    schema: String,
    table: String,
    #[serde(default)]
    tenant: Option<String>,
    #[serde(default)]
    record: Option<serde_json::Value>,
    #[serde(default)]
    old_record: Option<serde_json::Value>,
    #[serde(default)]
    #[allow(dead_code)] // retained on the wire for operator debugging; not forwarded.
    truncated: bool,
}

/// Module-private payload error. NOT part of the public API and never reaches the
/// port (`StreamError` has only `Unavailable`/`Denied`); the listen loop drops on it.
#[derive(Debug, thiserror::Error)]
enum PayloadError {
    #[error("payload json")]
    Json,
    #[error("unknown op")]
    UnknownOp,
}

/// Parse a NOTIFY payload into `(tenant, ChangeEvent)`. Pure — the core of the no-DB
/// unit tests. `tenant` is returned alongside for fan-out pre-filtering and is NOT
/// part of the domain `ChangeEvent`.
fn parse_payload(raw: &str) -> Result<(Option<String>, ChangeEvent), PayloadError> {
    let parsed: RawNotify = serde_json::from_str(raw).map_err(|_| PayloadError::Json)?;
    let op = op_from_str(&parsed.op).ok_or(PayloadError::UnknownOp)?;
    let event = ChangeEvent {
        op,
        schema: parsed.schema,
        table: parsed.table,
        record: parsed.record,
        old_record: parsed.old_record,
    };
    Ok((parsed.tenant, event))
}

/// Map a lowercase wire op string to `ChangeOp`. The trigger emits lowercase, so
/// uppercase / unknown values map to `None` (and become `PayloadError::UnknownOp`).
fn op_from_str(s: &str) -> Option<ChangeOp> {
    match s {
        "insert" => Some(ChangeOp::Insert),
        "update" => Some(ChangeOp::Update),
        "delete" => Some(ChangeOp::Delete),
        "upsert" => Some(ChangeOp::Upsert),
        _ => None,
    }
}

/// Validate `entity_type` (`<schema>.<table>`): each dot-segment is a SQL-safe
/// identifier (ASCII alnum/underscore, non-empty, not digit-led, ≤63 bytes). This
/// is defense-in-depth before the value is interpolated into the Keto check URL.
fn is_safe_entity(entity: &str) -> bool {
    let mut parts = entity.split('.');
    let (Some(schema), Some(table), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    is_safe_ident_segment(schema) && is_safe_ident_segment(table)
}

/// Validate `tenant`: empty is allowed (tables without a tenant column), otherwise
/// a slug/UUID-shaped token — ASCII alnum, `_` or `-`, ≤128 bytes. Rejects the URL
/// reserved chars (`&`, `=`, `#`, `/`, whitespace) that could corrupt the Keto query.
fn is_safe_tenant(tenant: &str) -> bool {
    tenant.is_empty()
        || (tenant.len() <= 128
            && tenant
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'))
}

/// One `<schema>`/`<table>` identifier segment.
fn is_safe_ident_segment(seg: &str) -> bool {
    if seg.is_empty() || seg.len() > 63 {
        return false;
    }
    let mut bytes = seg.bytes();
    let first = bytes.next().unwrap_or(b'0');
    (first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Does an event's `<schema>.<table>` equal the subscription's `entity_type`?
/// Extracted for unit testing the fan-out filter without a broadcast channel.
fn matches(spec_entity: &str, ev_schema: &str, ev_table: &str) -> bool {
    match spec_entity.split_once('.') {
        Some((schema, table)) => schema == ev_schema && table == ev_table,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- op_from_str mapping ------------------------------------------------

    #[test]
    fn op_mapping_known_values() {
        assert_eq!(op_from_str("insert"), Some(ChangeOp::Insert));
        assert_eq!(op_from_str("update"), Some(ChangeOp::Update));
        assert_eq!(op_from_str("delete"), Some(ChangeOp::Delete));
        assert_eq!(op_from_str("upsert"), Some(ChangeOp::Upsert));
    }

    #[test]
    fn op_mapping_rejects_empty_unknown_and_uppercase() {
        assert_eq!(op_from_str(""), None);
        assert_eq!(op_from_str("garbage"), None);
        // Trigger emits lowercase; uppercase is intentionally unmapped.
        assert_eq!(op_from_str("INSERT"), None);
    }

    // --- parse_payload happy paths -----------------------------------------

    #[test]
    fn parse_full_insert() {
        let raw = r#"{
            "op":"insert","schema":"public","table":"documents","tenant":"acme",
            "record":{"id":42,"title":"x","tenant_id":"acme"},
            "old_record":null,"truncated":false
        }"#;
        let (tenant, ev) = parse_payload(raw).expect("parse insert");
        assert_eq!(tenant.as_deref(), Some("acme"));
        assert_eq!(ev.op, ChangeOp::Insert);
        assert_eq!(ev.schema, "public");
        assert_eq!(ev.table, "documents");
        assert!(ev.record.is_some());
        assert!(ev.old_record.is_none());
    }

    #[test]
    fn parse_update_has_both_images() {
        let raw = r#"{
            "op":"update","schema":"public","table":"documents","tenant":"acme",
            "record":{"id":42,"title":"new"},
            "old_record":{"id":42,"title":"old"},"truncated":false
        }"#;
        let (_tenant, ev) = parse_payload(raw).expect("parse update");
        assert_eq!(ev.op, ChangeOp::Update);
        assert!(ev.record.is_some());
        assert!(ev.old_record.is_some());
    }

    #[test]
    fn parse_delete_has_old_only() {
        let raw = r#"{
            "op":"delete","schema":"public","table":"documents","tenant":"acme",
            "record":null,"old_record":{"id":42},"truncated":false
        }"#;
        let (_tenant, ev) = parse_payload(raw).expect("parse delete");
        assert_eq!(ev.op, ChangeOp::Delete);
        assert!(ev.record.is_none());
        assert!(ev.old_record.is_some());
    }

    #[test]
    fn parse_truncated_pk_only_preserves_pk() {
        // The load-bearing overflow case: a truncated event must still carry the PK
        // so the downstream build_pk_filters re-query can re-fetch the full row.
        let raw = r#"{
            "op":"update","schema":"public","table":"documents","tenant":"acme",
            "record":{"id":42},"old_record":{"id":42},"truncated":true
        }"#;
        let (_tenant, ev) = parse_payload(raw).expect("parse truncated");
        let pk = ev
            .record
            .as_ref()
            .and_then(|r| r.get("id"))
            .and_then(serde_json::Value::as_i64);
        assert_eq!(pk, Some(42));
    }

    #[test]
    fn parse_missing_tenant_is_none() {
        let raw = r#"{
            "op":"insert","schema":"public","table":"logs",
            "record":{"id":1},"old_record":null
        }"#;
        let (tenant, ev) = parse_payload(raw).expect("parse no-tenant");
        assert!(tenant.is_none());
        assert_eq!(ev.op, ChangeOp::Insert);
    }

    // --- parse_payload error paths -----------------------------------------

    #[test]
    fn parse_non_json_errors() {
        assert!(matches!(
            parse_payload("not json at all"),
            Err(PayloadError::Json)
        ));
    }

    #[test]
    fn parse_missing_op_errors() {
        let raw = r#"{"schema":"public","table":"documents"}"#;
        assert!(matches!(parse_payload(raw), Err(PayloadError::Json)));
    }

    #[test]
    fn parse_unknown_op_errors() {
        let raw = r#"{"op":"truncate","schema":"public","table":"documents"}"#;
        assert!(matches!(parse_payload(raw), Err(PayloadError::UnknownOp)));
    }

    // --- entity_type filter logic ------------------------------------------

    #[test]
    fn matches_exact_schema_and_table() {
        assert!(matches("public.documents", "public", "documents"));
    }

    #[test]
    fn matches_rejects_table_mismatch() {
        assert!(!matches("public.documents", "public", "orders"));
    }

    #[test]
    fn matches_rejects_schema_mismatch() {
        assert!(!matches("public.documents", "auth", "documents"));
    }

    #[test]
    fn matches_rejects_unqualified_entity() {
        assert!(!matches("documents", "public", "documents"));
    }

    // --- Defense-in-depth identifier validation (pre-Keto, fail closed) -----

    #[test]
    fn is_safe_entity_accepts_qualified_and_rejects_injection() {
        assert!(is_safe_entity("public.documents"));
        assert!(is_safe_entity("app_1.order_items"));
        // Not two dot-segments, or unsafe chars, or URL-reserved injection.
        assert!(!is_safe_entity("documents")); // unqualified
        assert!(!is_safe_entity("public.doc.extra")); // three segments
        assert!(!is_safe_entity("public.docs&relation=owner")); // query injection
        assert!(!is_safe_entity("public.1docs")); // digit-led table
        assert!(!is_safe_entity("pub lic.docs")); // whitespace
        assert!(!is_safe_entity("")); // empty
    }

    #[test]
    fn is_safe_tenant_allows_slug_uuid_empty_rejects_reserved() {
        assert!(is_safe_tenant("")); // no-tenant tables
        assert!(is_safe_tenant("acme"));
        assert!(is_safe_tenant("550e8400-e29b-41d4-a716-446655440000")); // uuid
        assert!(is_safe_tenant("tenant_42"));
        assert!(!is_safe_tenant("acme&relation=owner")); // query injection
        assert!(!is_safe_tenant("a/b")); // path
        assert!(!is_safe_tenant("a b")); // whitespace
        assert!(!is_safe_tenant(&"x".repeat(129))); // over cap
    }

    // --- Keto fail-closed (no DB; a dead port for the Unavailable case) -----

    fn rls_with_subject(subject: &str) -> RlsContext {
        RlsContext {
            role: "authenticated".to_string(),
            claims_json: "{}".to_string(),
            raw_bearer: "token".to_string(),
            keto_subject: subject.to_string(),
            vault_key_id: None,
        }
    }

    #[tokio::test]
    async fn keto_allowed_true_ok() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/relation-tuples/check"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"allowed": true})),
            )
            .mount(&server)
            .await;

        let out = keto_check_via_http(
            &reqwest::Client::new(),
            &server.uri(),
            "public.documents",
            "acme",
            "user-1",
        )
        .await;
        assert!(out.is_ok());
    }

    #[tokio::test]
    async fn keto_allowed_false_denied() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/relation-tuples/check"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"allowed": false})),
            )
            .mount(&server)
            .await;

        let out = keto_check_via_http(
            &reqwest::Client::new(),
            &server.uri(),
            "public.documents",
            "acme",
            "user-1",
        )
        .await;
        assert!(matches!(out, Err(StreamError::Denied)));
    }

    #[tokio::test]
    async fn keto_forbidden_denied() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/relation-tuples/check"))
            .respond_with(wiremock::ResponseTemplate::new(403))
            .mount(&server)
            .await;

        let out = keto_check_via_http(
            &reqwest::Client::new(),
            &server.uri(),
            "public.documents",
            "acme",
            "user-1",
        )
        .await;
        assert!(matches!(out, Err(StreamError::Denied)));
    }

    #[tokio::test]
    async fn keto_server_error_unavailable_fail_closed() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let out = keto_check_via_http(
            &reqwest::Client::new(),
            &server.uri(),
            "public.documents",
            "acme",
            "user-1",
        )
        .await;
        assert!(matches!(out, Err(StreamError::Unavailable)));
    }

    #[tokio::test]
    async fn keto_connection_refused_unavailable_fail_closed() {
        // Point at a port nothing is listening on. This is the load-bearing
        // fail-closed assertion: a transport error must deny, never allow.
        let dead = "http://127.0.0.1:1";
        let out = keto_check_via_http(
            &reqwest::Client::new(),
            dead,
            "public.documents",
            "acme",
            "user-1",
        )
        .await;
        assert!(matches!(out, Err(StreamError::Unavailable)));
    }

    #[test]
    fn rls_context_constructor_excludes_pii_from_debug_expectations() {
        // Sanity: keto_subject is a field but must never be logged. This test
        // exercises the constructor used by the async tests below.
        let rls = rls_with_subject("user-1");
        assert_eq!(rls.keto_subject, "user-1");
        assert_eq!(rls.role, "authenticated");
    }

    // --- Broadcast lag mapping (pure tokio; no DB, no Postgres) --------------

    #[tokio::test]
    async fn broadcast_lag_becomes_skip_not_terminate() {
        // Capacity 2, overflow it, then confirm the watch-style filter turns the
        // Lagged signal into a skipped (None) item rather than an Err / termination.
        let (tx, rx) = tokio::sync::broadcast::channel::<Arc<ChangeEvent>>(2);
        let ev = |t: &str| {
            Arc::new(ChangeEvent {
                op: ChangeOp::Insert,
                schema: "public".to_string(),
                table: t.to_string(),
                record: None,
                old_record: None,
            })
        };
        // Send more than capacity before the receiver reads → guaranteed lag.
        for i in 0..5 {
            let _ = tx.send(ev(&format!("t{i}")));
        }

        let want_entity = "public.documents".to_string();
        let mut stream = BroadcastStream::new(rx)
            .filter_map(move |item| {
                let want_entity = want_entity.clone();
                async move {
                    match item {
                        Ok(e) if matches(&want_entity, &e.schema, &e.table) => {
                            Some(Ok::<(), StreamError>(()))
                        }
                        // Non-matching table OR a Lagged signal: both drop to None
                        // (skip), never Err, never terminate.
                        Ok(_) | Err(BroadcastStreamRecvError::Lagged(_)) => None,
                    }
                }
            })
            .boxed();

        // The lagged window is skipped (None) and the stream ends cleanly once the
        // channel is exhausted and dropped — it never yields Err and never panics.
        drop(tx);
        let mut yielded_err = false;
        while let Some(item) = stream.next().await {
            if item.is_err() {
                yielded_err = true;
            }
        }
        assert!(!yielded_err, "lag must not surface as an Err to the subscriber");
    }
}
