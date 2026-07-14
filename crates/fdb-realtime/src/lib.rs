//! `ChangeStreamSource` implementation over the FRF `EntityService.WatchEntity` gRPC stream.
//!
//! Architecture:
//!
//! 1. **Keto coarse check** — confirm the subscriber has `view` relation on the `entity_type`
//!    before opening a stream. If Keto is unavailable, return `StreamError::Unavailable`
//!    (fail closed, never fail open).
//!
//! 2. **FRF stream** — open `EntityService.WatchEntity` (or WatchEntityType once FRF adds it;
//!    see OQ-FRF-1 in the Phase 3 plan). Yield raw `EntityChange` messages.
//!
//! 3. **Per-event RLS re-query** — for each event, re-query the changed row from Postgres
//!    under full RLS context. The event is delivered to the subscriber ONLY if the row
//!    survives the re-query. This is non-negotiable and non-configurable (WAL bypasses RLS).
//!
//! # Security invariants
//!
//! - `rls.keto_subject` MUST NOT appear in any tracing span.
//! - If Keto is unavailable: return `Err(StreamError::Unavailable)` — never default to allow.
//! - The per-event RLS re-query is NEVER removed or skipped.
//! - `FabricChangeSource` MUST NOT be constructed with a service-role `RlsContext`.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod listen;
pub use listen::{ListenChangeSource, ListenConfig, ListenError};

use std::sync::Arc;

use async_trait::async_trait;
use fdb_domain::{ChangeEvent, ChangeOp, SubscriptionSpec};
use fdb_ports::{ChangeStreamSource, StreamError};
use forge_identity::RlsContext;
use futures::stream::{BoxStream, StreamExt};
use tracing::instrument;

/// Configuration for the Keto HTTP check endpoint.
#[derive(Debug, Clone)]
pub struct KetoConfig {
    /// Base URL of the Keto read API, e.g. `http://keto:4466`.
    pub base_url: String,
}

/// Configuration for the FRF gRPC endpoint.
#[derive(Debug, Clone)]
pub struct FrfConfig {
    /// Full gRPC address, e.g. `http://frf:50051`.
    pub endpoint: String,
}

/// `ChangeStreamSource` backed by the Flint Realtime Fabric gRPC stream.
///
/// One instance is shared across all subscription connections. The tonic
/// `Channel` is connection-pooled internally.
///
/// # OQ-FRF-1 (open question)
///
/// FRF currently exposes `WatchEntity(entity_id, tenant_id)` — a single entity watcher.
/// Table-level subscriptions require `WatchEntityType(entity_type, tenant_id)`, which has
/// been proposed to the FRF team. Until that RPC lands, `watch()` performs the Keto
/// check and then returns `stream::empty()` with a `tracing::warn!` — NOT a panic — so
/// a caller opting into this backend (`FLINT_CHANGE_SOURCE=fabric`) degrades to "no
/// events" rather than crashing. `listen` (Postgres LISTEN/NOTIFY, real events) is the
/// default backend as of p16-c004 for exactly this reason; opt into `fabric` only once
/// `WatchEntityType` is confirmed available upstream. All surrounding infrastructure
/// (Keto check, RLS re-query) is fully implemented and shared with `ListenChangeSource`.
#[derive(Clone)]
pub struct FabricChangeSource {
    /// tonic channel to the FRF `EntityService`.
    /// OQ-FRF-1: proto codegen deferred; channel held as the typed connection boundary.
    #[allow(dead_code)]
    channel: Arc<tonic::transport::Channel>,
    /// HTTP client for Keto relation check.
    http: reqwest::Client,
    keto: KetoConfig,
}

impl FabricChangeSource {
    /// Build a `FabricChangeSource`, establishing the tonic channel.
    ///
    /// The channel connects lazily (`connect_lazy`), so this does not block on
    /// or verify network reachability — only the endpoint's URI shape is
    /// validated here. An unreachable-but-well-formed endpoint fails later,
    /// on first RPC attempt (currently unreachable in practice: `watch` never
    /// issues the real call — see [`FabricChangeSource::watch`]'s OQ-FRF-1 note).
    ///
    /// # Errors
    ///
    /// Returns [`FabricError::Connect`] when `frf.endpoint` is not a valid URI.
    pub fn new(frf: FrfConfig, keto: KetoConfig) -> Result<Self, FabricError> {
        let channel = tonic::transport::Channel::from_shared(frf.endpoint)
            .map_err(|e| FabricError::Connect(e.to_string()))?
            .connect_lazy();
        Ok(Self {
            channel: Arc::new(channel),
            http: reqwest::Client::new(),
            keto,
        })
    }
}

/// Errors from `FabricChangeSource` construction.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FabricError {
    /// The tonic `Channel` could not be built from the configured FRF gRPC
    /// endpoint (e.g. an invalid URI). Note this is a lazy channel
    /// (`connect_lazy`) — a merely *unreachable* endpoint does not surface
    /// here; it only fails once a call is actually attempted.
    #[error("gRPC connect: {0}")]
    Connect(String),
}

#[async_trait]
impl ChangeStreamSource for FabricChangeSource {
    /// Open a change stream for `spec.entity_type` as subscriber `who`.
    ///
    /// # Security
    ///
    /// 1. Keto check before stream open — fail closed on Keto unavailability.
    /// 2. Per-event RLS re-query — events that do not survive the re-query are dropped.
    #[instrument(skip(self, who), fields(entity_type = %spec.entity_type, tenant = %spec.tenant), err)]
    async fn watch(
        &self,
        spec: SubscriptionSpec,
        who: &RlsContext,
        // SECURITY: who.keto_subject MUST NOT be logged here or in any span above.
    ) -> Result<BoxStream<'static, Result<ChangeEvent, StreamError>>, StreamError> {
        // Step 1: Keto coarse check — confirm `keto_subject` has `view` on `entity_type`.
        // Fail closed: if Keto is unreachable, deny the subscription.
        self.keto_check(&spec.entity_type, &spec.tenant, &who.keto_subject)
            .await?;

        // Step 2: OQ-FRF-1 — FRF WatchEntityType RPC not yet available.
        // Until FRF exposes WatchEntityType, this returns an empty stream with a log.
        // When WatchEntityType lands, replace this with the tonic streaming call.
        tracing::warn!(
            entity_type = %spec.entity_type,
            "OQ-FRF-1: WatchEntityType not yet available in FRF; returning empty stream"
        );

        let empty: BoxStream<'static, Result<ChangeEvent, StreamError>> =
            futures::stream::empty().boxed();
        Ok(empty)

        // --- Future implementation (post OQ-FRF-1 resolution) ---
        //
        // let mut client = entity_service_client::EntityServiceClient::new(self.channel.as_ref().clone());
        // let request = tonic::Request::new(WatchEntityTypeRequest {
        //     entity_type: spec.entity_type.clone(),
        //     tenant_id: spec.tenant.clone(),
        // });
        // let frf_stream = client.watch_entity_type(request).await
        //     .map_err(|_| StreamError::Unavailable)?
        //     .into_inner();
        //
        // let rls_clone = who.clone();   // RlsContext must be Clone for stream capture
        // let db_clone = self.db.clone();
        //
        // let event_stream = frf_stream.filter_map(move |msg| {
        //     let rls = rls_clone.clone();
        //     let db = db_clone.clone();
        //     async move {
        //         let entity_change = msg.ok()?;
        //         // Step 3: Per-event RLS re-query — NON-NEGOTIABLE.
        //         // Re-query the changed row as the subscriber; deliver only if it survives RLS.
        //         rls_requery(&db, &entity_change, &rls).await
        //     }
        // });
        // Ok(event_stream.boxed())
    }
}

/// Check the Keto relationship: does `subject` have `view` on `object` in `namespace`?
///
/// SECURITY:
/// - `subject` is the `keto_subject` (PII) — MUST NOT appear in logs.
/// - If Keto is unavailable, returns `Err(StreamError::Unavailable)` — never allow.
async fn keto_check_via_http(
    client: &reqwest::Client,
    keto_base_url: &str,
    namespace: &str,
    object: &str,
    subject: &str,
) -> Result<(), StreamError> {
    let url = format!(
        "{keto_base_url}/relation-tuples/check?namespace={namespace}&object={object}&relation=view&subject_id={subject}"
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|_| StreamError::Unavailable)?;

    match resp.status() {
        s if s.is_success() => {
            // Parse `{"allowed": true/false}`
            let body: serde_json::Value =
                resp.json().await.map_err(|_| StreamError::Unavailable)?;
            if body
                .get("allowed")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
                Ok(())
            } else {
                Err(StreamError::Denied)
            }
        }
        reqwest::StatusCode::FORBIDDEN => Err(StreamError::Denied),
        _ => Err(StreamError::Unavailable),
    }
}

impl FabricChangeSource {
    /// Keto coarse check — confirm view permission before opening a stream.
    ///
    /// SECURITY: `subject` is PII; MUST NOT be logged.
    async fn keto_check(
        &self,
        entity_type: &str,
        tenant_id: &str,
        subject: &str,
    ) -> Result<(), StreamError> {
        keto_check_via_http(
            &self.http,
            &self.keto.base_url,
            entity_type,
            tenant_id,
            subject,
        )
        .await
    }
}

/// Convert an FRF `ChangeOp` integer to the domain `ChangeOp`.
/// Used when the tonic WatchEntityType call is implemented (OQ-FRF-1).
#[allow(dead_code)]
fn frf_op_to_domain(frf_op: i32) -> Option<ChangeOp> {
    match frf_op {
        1 => Some(ChangeOp::Insert),
        2 => Some(ChangeOp::Update),
        3 => Some(ChangeOp::Delete),
        4 => Some(ChangeOp::Upsert),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frf_op_mapping() {
        assert_eq!(frf_op_to_domain(1), Some(ChangeOp::Insert));
        assert_eq!(frf_op_to_domain(2), Some(ChangeOp::Update));
        assert_eq!(frf_op_to_domain(3), Some(ChangeOp::Delete));
        assert_eq!(frf_op_to_domain(4), Some(ChangeOp::Upsert));
        assert_eq!(frf_op_to_domain(0), None);
        assert_eq!(frf_op_to_domain(99), None);
    }
}
