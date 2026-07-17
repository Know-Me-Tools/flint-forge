use async_trait::async_trait;
use fdb_domain::{ChangeEvent, SubscriptionSpec};
use fdb_ports::{ChangeStreamSource, StreamError};
use forge_identity::RlsContext;
use futures::stream::{BoxStream, StreamExt};
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;
use tracing::instrument;

use super::source::ListenChangeSource;
use super::validate::{is_safe_entity, is_safe_tenant, matches};
use crate::keto_check_via_http;

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
