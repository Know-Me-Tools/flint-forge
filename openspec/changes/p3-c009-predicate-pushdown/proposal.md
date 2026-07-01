# p3-c009 — Predicate Pushdown: Opt-In RLS Pre-Filter for Subscriptions

## Change ID
`p3-c009-predicate-pushdown`

## Phase
`p3-graphql-hybrid-engine`

## Priority
P2 — Deferred optimization. Requires p3-c002 (subscriptions) to be complete.
Do NOT implement before p3-c002 is fully tested and production-verified.

## ⚠️ OPERATOR RISK ACKNOWLEDGMENT REQUIRED

This change introduces an optimization that carries an **accepted data-leak
risk**. It MUST NOT be enabled without:

1. Explicit operator consent in writing
2. A deployment flag (`FLINT_PREDICATE_PUSHDOWN=true`) — disabled by default
3. Documentation in `docs/operations/predicate-pushdown.md` describing the
   risk and the conditions under which it is safe

**The risk**: If the pushed-down predicate does not exactly match the
server-side RLS policy, events that should be hidden from the subscriber may
be delivered. This is a data confidentiality risk. The per-event RLS re-query
(p3-c002) is the authoritative gate; predicate pushdown is a throughput
optimization that bypasses the expensive re-query when the operator has
determined the risk is acceptable.

## Problem Statement

For high-throughput subscriptions (e.g., 10,000+ events/second across a large
tenant), the per-event RLS re-query (p3-c002 T5) creates a Postgres connection
per event. This does not scale without connection pooling limits being hit.

Predicate pushdown allows passing a filter predicate to `WatchEntityType` so
that FRF only delivers events the subscriber is likely to see, reducing
re-query volume.

## Scope

### In Scope
- `SubscriptionSpec` gains an optional `predicate: Option<String>` field
- `FabricChangeSource::watch()` passes the predicate to `WatchEntityTypeRequest`
  when `FLINT_PREDICATE_PUSHDOWN=true` AND a predicate is provided
- The per-event RLS re-query is NOT removed — it remains mandatory
- An operator env var `FLINT_PREDICATE_PUSHDOWN` gates the feature (default: false)
- A warning log is emitted at startup when predicate pushdown is enabled

### Out of Scope
- Removing or weakening the per-event RLS re-query (it is never removed)
- Predicate syntax validation (delegated to FRF)
- User-supplied predicates (predicates come from application layer, not user input)

## Design

### SubscriptionSpec change (fdb-ports)

```rust
pub struct SubscriptionSpec {
    pub entity_type: String,
    pub tenant_id: String,
    pub filter: Option<serde_json::Value>,
    // NEW:
    pub predicate: Option<String>,  // opt-in; only honored when FLINT_PREDICATE_PUSHDOWN=true
}
```

### FabricChangeSource::watch() change (fdb-realtime)

```rust
let effective_predicate = if std::env::var("FLINT_PREDICATE_PUSHDOWN").as_deref() == Ok("true") {
    spec.predicate.as_deref()
} else {
    None  // predicate silently ignored when feature is off
};

let request = WatchEntityTypeRequest {
    tenant_id: spec.tenant_id.clone(),
    entity_type: spec.entity_type.clone(),
    filter: spec.filter.clone(),
    predicate: effective_predicate.map(|s| s.to_string()),  // new field, if FRF supports it
};
```

### Startup warning

In `fdb-gateway/src/main.rs` (startup block):
```rust
if std::env::var("FLINT_PREDICATE_PUSHDOWN").as_deref() == Ok("true") {
    tracing::warn!(
        "FLINT_PREDICATE_PUSHDOWN is enabled. \
         This optimization trades throughput for a potential data confidentiality risk. \
         The per-event RLS re-query remains active but filtered events reduce re-query volume. \
         See docs/operations/predicate-pushdown.md."
    );
}
```

## Security Contracts (CRITICAL)
- The per-event RLS re-query from p3-c002 is NEVER removed or skipped, even
  when predicate pushdown is enabled
- Predicates are only passed to FRF by Quarry's application layer — they are
  NOT derived from user input or subscription arguments
- This feature is gated by `FLINT_PREDICATE_PUSHDOWN=true` (not a runtime API)
- A warning MUST be logged at startup when this feature is enabled
- Documentation in `docs/operations/predicate-pushdown.md` must describe the
  risk before this feature can be deployed

## Acceptance Criteria
- `SubscriptionSpec.predicate` field added
- `FabricChangeSource::watch()` passes predicate to FRF when feature flag is set
- `FLINT_PREDICATE_PUSHDOWN` env var controls the feature (default: false)
- Startup warning log emitted when feature is enabled
- `docs/operations/predicate-pushdown.md` written describing the risk
- Unit test `test_predicate_not_sent_when_feature_flag_off`: verify predicate
  is NOT included in `WatchEntityTypeRequest` when flag is unset
- Per-event RLS re-query still called in all cases (verified by existing test
  from p3-c002 T6)
- `cargo check --workspace` GREEN; clippy pedantic passes
