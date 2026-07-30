//! Flint Quarry application layer — use-cases composed against ports.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod a2ui;
pub mod graphql;
pub mod provision;

use fdb_domain::{ChangeEvent, RestQuery, SubscriptionSpec, TableMeta};
use fdb_ports::{BackendError, ChangeStreamSource, GraphQlExecutor, RestExecutor, StreamError};
use forge_domain::Json;
use forge_identity::RlsContext;
use futures::stream::{BoxStream, StreamExt};
use std::sync::Arc;

/// Wires the use-cases over whatever adapters the interface layer injects.
///
/// # Authorization does not live here
///
/// `Quarry` carries no gate of its own. Both live gates sit in
/// `fdb-reflection`'s `mutation_guard`, which consults Keto and Cedar through
/// `RestState`; subscriptions are authorized by the change source's own
/// subscribe-time check plus the per-event RLS re-query in
/// [`Quarry::subscribe_rls_filtered`].
///
/// A Cedar `pep` field with `with_pep`/`check_pep` accessors used to live here
/// and was removed in p17. The gateway's only `Quarry::new` call
/// (`subscriptions.rs`) never attached a PEP, so the field was permanently
/// `None` and the check unreachable — the same "gate that looks like
/// enforcement but never runs" shape as the Keto cluster removed alongside it.
pub struct Quarry {
    /// REST query/mutation executor adapter (Postgres-backed in production).
    pub rest: Arc<dyn RestExecutor>,
    /// GraphQL Query/Mutation executor adapter, delegating to `graphql.resolve()`.
    pub graphql: Arc<dyn GraphQlExecutor>,
    /// Change-stream source for subscriptions (gRPC client of the realtime fabric).
    pub changes: Arc<dyn ChangeStreamSource>,
}

/// Error returned by subscription use-cases.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SubscriptionError {
    /// The underlying change stream failed.
    #[error(transparent)]
    Stream(#[from] StreamError),
    /// The RLS re-query failed.
    #[error(transparent)]
    Backend(#[from] BackendError),
}

impl Quarry {
    /// Construct a `Quarry` from the three mandatory adapters.
    ///
    /// There is deliberately no authorization gate here — neither Keto nor
    /// Cedar. Both live gates sit in `fdb_reflection`'s `mutation_guard`:
    ///
    /// - Keto is checked as
    ///   `(entities, "<schema>.<table>", insert|update|delete, subject)`.
    ///   `Quarry` once carried a parallel gate keyed on
    ///   `(entities, <bare table>, "mutate", subject)` — an incompatible tuple
    ///   shape that no production path ever reached, so tuples seeded against
    ///   it could never match.
    /// - Cedar is consulted through `RestState::pep`. `Quarry` once carried its
    ///   own `pep` field with `with_pep`/`check_pep` accessors, but the
    ///   gateway's only construction site (`subscriptions.rs`) never attached
    ///   one, leaving the field permanently `None` and the check unreachable.
    ///
    /// Both were removed in p17 rather than left as a second, unreachable
    /// vocabulary that reads like enforcement.
    pub fn new(
        rest: Arc<dyn RestExecutor>,
        graphql: Arc<dyn GraphQlExecutor>,
        changes: Arc<dyn ChangeStreamSource>,
    ) -> Self {
        Self {
            rest,
            graphql,
            changes,
        }
    }

    /// Subscribe to a change stream with per-event RLS re-query filtering.
    ///
    /// For each `ChangeEvent` from the underlying `ChangeStreamSource`, the
    /// changed row is re-queried through the `RestExecutor` under the
    /// subscriber's RLS context. Events that do not survive the re-query
    /// (zero rows returned) are silently dropped. This is non-negotiable:
    /// WAL replication bypasses Postgres RLS, so the re-query is the
    /// authoritative visibility check.
    pub async fn subscribe_rls_filtered(
        &self,
        spec: SubscriptionSpec,
        table_meta: TableMeta,
        who: &RlsContext,
    ) -> Result<BoxStream<'static, Result<ChangeEvent, SubscriptionError>>, SubscriptionError> {
        let stream = self.changes.watch(spec, who).await?;
        let rest = self.rest.clone();
        let who = who.clone();
        Ok(stream
            .filter_map(move |res| {
                let rest = rest.clone();
                let who = who.clone();
                let table_meta = table_meta.clone();
                async move {
                    match res {
                        Ok(event) => match build_pk_filters(&event, &table_meta) {
                            Some(filters) => {
                                let query = RestQuery {
                                    schema: event.schema.clone(),
                                    table: event.table.clone(),
                                    select: None,
                                    filters,
                                    order: None,
                                    limit: Some(1),
                                    offset: None,
                                };
                                match rest.execute(query, &who).await {
                                    Ok(result) => match result.rows {
                                        Json::Array(rows) if !rows.is_empty() => Some(Ok(event)),
                                        _ => None,
                                    },
                                    Err(e) => Some(Err(SubscriptionError::Backend(e))),
                                }
                            }
                            None => None,
                        },
                        Err(e) => Some(Err(SubscriptionError::Stream(e))),
                    }
                }
            })
            .boxed())
    }
}

impl Quarry {
    /// Subscribe to a table's change stream, RLS-filtered, projected to
    /// `async_graphql::Value` objects ready for a `graphql-transport-ws` field.
    ///
    /// This is the seam the gateway's subscription-stream factory calls. Each
    /// surviving `ChangeEvent` becomes an object with the record's columns plus a
    /// synthetic `_op` field naming the operation. Errors from the underlying
    /// RLS-filtered stream are surfaced as GraphQL errors (terminating the field
    /// stream), never silently swallowed.
    ///
    /// SECURITY: `who` carries `keto_subject` (PII) — never logged. Visibility is
    /// enforced by [`Quarry::subscribe_rls_filtered`]'s per-event re-query; this
    /// method only projects events that already survived that check.
    pub async fn subscribe_graphql_values(
        &self,
        spec: SubscriptionSpec,
        table_meta: TableMeta,
        who: &RlsContext,
    ) -> Result<BoxStream<'static, async_graphql::Result<async_graphql::Value>>, SubscriptionError>
    {
        let stream = self.subscribe_rls_filtered(spec, table_meta, who).await?;
        Ok(stream
            .map(|res| match res {
                Ok(event) => Ok(change_event_to_value(&event)),
                Err(e) => Err(async_graphql::Error::new(e.to_string())),
            })
            .boxed())
    }
}

/// Project a `ChangeEvent` into an `async_graphql::Value` object: the record's
/// columns plus a synthetic `_op` field. A delete event with no `record` falls
/// back to `old_record` so subscribers still see the removed row's key columns.
fn change_event_to_value(event: &ChangeEvent) -> async_graphql::Value {
    use async_graphql::{Name, Value};
    use std::collections::BTreeMap;

    let mut map: async_graphql::indexmap::IndexMap<Name, Value> =
        async_graphql::indexmap::IndexMap::new();
    map.insert(Name::new("_op"), Value::from(change_op_str(event.op)));

    let source = event.record.as_ref().or(event.old_record.as_ref());
    if let Some(Json::Object(obj)) = source {
        // Preserve a stable field order for deterministic output.
        let ordered: BTreeMap<&String, &Json> = obj.iter().collect();
        for (k, v) in ordered {
            map.insert(
                Name::new(k),
                Value::from_json(v.clone()).unwrap_or(Value::Null),
            );
        }
    }
    Value::Object(map)
}

/// Stable lowercase operation marker for the synthetic `_op` field.
fn change_op_str(op: fdb_domain::ChangeOp) -> &'static str {
    match op {
        fdb_domain::ChangeOp::Insert => "insert",
        fdb_domain::ChangeOp::Update => "update",
        fdb_domain::ChangeOp::Delete => "delete",
        fdb_domain::ChangeOp::Upsert => "upsert",
        // `ChangeOp` is #[non_exhaustive]; a future variant maps to a neutral marker
        // rather than panicking on a live subscription path.
        _ => "unknown",
    }
}

/// Extract primary-key equality filters from a change event's record.
///
/// Returns `None` when the record is missing, is not a JSON object, or is
/// missing any primary-key column.
fn build_pk_filters(
    event: &ChangeEvent,
    table_meta: &TableMeta,
) -> Option<Vec<(String, String, String)>> {
    let record = event.record.as_ref()?;
    let obj = record.as_object()?;
    table_meta
        .primary_key
        .iter()
        .map(|pk| {
            let value = obj.get(pk).and_then(json_value_to_filter)?;
            Some((pk.clone(), "eq".to_string(), value))
        })
        .collect::<Option<Vec<_>>>()
}

/// Convert a scalar JSON value into a REST filter value string.
fn json_value_to_filter(value: &Json) -> Option<String> {
    match value {
        Json::String(s) => Some(s.clone()),
        Json::Number(n) => Some(n.to_string()),
        Json::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}
