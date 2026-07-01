# p3-c002 — Subscriptions: FRF WatchEntityType + Keto Gate + Per-Event RLS Re-Query

## Change ID
`p3-c002-subscriptions`

## Phase
`p3-graphql-hybrid-engine`

## Priority
P0 — Requires p3-c004 (WebSocket handler) and FRF WatchEntityType gRPC service (OQ-FRF-1)

## Problem Statement

`FabricChangeSource::watch()` is `todo!()`. No gRPC connection to FRF
`WatchEntityType` exists. No Keto check is performed. The non-negotiable
per-event RLS re-query (CLAUDE.md §Subscription RLS Enforcement) is not
implemented.

## Scope

### In Scope
- Add `tonic` to workspace `Cargo.toml` and `fdb-realtime/Cargo.toml`
- Resolve OQ-FRF-1: locate the FRF WatchEntityType `.proto` in `flint-realtime-fabric`
  and generate or import the tonic client
- Implement `FabricChangeSource` struct fields: tonic channel, Keto HTTP client, PgPool for re-query
- Implement `FabricChangeSource::watch()`:
  1. Keto coarse check: `check(rls.keto_subject, "view", spec.entity_type)` — cached per connection
  2. Call `WatchEntityType(tenant_id, entity_type, filter)` gRPC stream
  3. For each `EntityChange` event: re-query `SELECT * FROM <schema>.<table> WHERE pk = $1`
     on a connection acquired via `DatabaseBackend::acquire(who)` (full 6-GUC RLS context)
  4. If row returned: map to `ChangeEvent` and yield
  5. If row absent (RLS filtered it out): skip — do NOT deliver to subscriber
- Connect `FabricChangeSource` to the async-graphql subscription resolvers via `ChangeStreamSource` trait

### Out of Scope
- The WebSocket upgrade handler itself (p3-c004)
- Keto tuple sync from FRF Iggy (p3-c006)
- Predicate-pushdown optimization (p3-c009, P2)

## Design

### FabricChangeSource struct (`fdb-realtime/src/lib.rs`)

```rust
pub struct FabricChangeSource {
    channel: tonic::transport::Channel,
    keto_base_url: String,
    http: reqwest::Client,
    db: Arc<dyn DatabaseBackend>,
    schema_name: String,
}
```

### FabricChangeSource::watch() — algorithm

```
1. keto_check(rls.keto_subject, "view", spec.entity_type, keto_base_url)
   → Err(Denied) if check fails
   → cache result for lifetime of subscription (subscribe-time gate)

2. channel.watch_entity_type(WatchEntityTypeRequest {
       tenant_id: spec.tenant_id,
       entity_type: spec.entity_type.clone(),
       filter: spec.filter.clone(),
   })
   → grpc_stream

3. for each EntityChange in grpc_stream:
   conn = db.acquire(who).await?
   row = query_one("SELECT * FROM {schema}.{table} WHERE {pk} = $1", pk_value, conn)
   if row.is_some(): yield Ok(ChangeEvent::from(row, entity_change.op))
   else: continue  // RLS filtered — do not deliver

4. On grpc_stream error: yield Err(StreamError::Unavailable); caller reconnects
```

### Per-event RLS re-query table name safety

The table name in `SELECT * FROM {schema}.{table}` MUST be constructed from
`spec.entity_type` mapped through a whitelist derived from `DatabaseModel` — NOT
from free-form user input. This prevents SQL injection via subscription entity_type.

### Keto inline check

```rust
async fn keto_check(subject: &str, verb: &str, object: &str, base_url: &str, client: &reqwest::Client)
    -> Result<(), StreamError>
{
    let resp = client
        .get(format!("{base_url}/relation-tuples/check"))
        .query(&[("namespace", "resources"), ("object", object), ("relation", verb), ("subject_id", subject)])
        .send().await
        .map_err(|_| StreamError::Unavailable)?;
    if resp.status().is_success() { Ok(()) } else { Err(StreamError::Denied) }
}
```

## Security Contracts (NON-NEGOTIABLE — CLAUDE.md §Subscription RLS Enforcement)

**Per-event RLS re-query is MANDATORY and NOT configurable:**
- Every `EntityChange` event MUST trigger a re-query of the changed row under the subscriber's `RlsContext`
- If the re-query returns no row: the event MUST be silently dropped (not delivered, not logged as an error)
- This contract cannot be disabled at runtime without operator-level code change + restart
- Predicate-pushdown (p3-c009) is opt-in and requires explicit operator acknowledgment of data-leak risk

**Keto check:**
- Keto check is performed ONCE at subscribe-time, not per-event
- Keto result MUST NOT be cached across different subscriber sessions
- If Keto is unavailable: return `StreamError::Unavailable` (do NOT default to allow)

**Subject logging:**
- `rls.keto_subject` MUST NOT appear in tracing spans (it is PII)
- Entity type and operation type may be logged (they are schema metadata, not PII)

## Acceptance Criteria
- `tonic` in workspace `Cargo.toml` and `fdb-realtime/Cargo.toml`
- `FabricChangeSource` struct fields populated (no more `/* tonic channel... */` comment)
- `FabricChangeSource::watch()` compiles without `todo!()`
- Per-event RLS re-query implemented in the watch loop
- Keto inline check implemented
- Unit test `test_watch_drops_rls_filtered_events` (mock: simulate re-query returning None; assert no event delivered)
- `cargo check --workspace` GREEN; clippy pedantic passes
