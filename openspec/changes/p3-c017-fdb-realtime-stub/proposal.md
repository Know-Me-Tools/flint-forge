# p3-c017 — fdb-realtime Production Stub (Reconnect + Service-Token Auth)

## Change ID
`p3-c017-fdb-realtime-stub`

## Phase
`p3-auth-rls-keto`

## Goal Mapping
**G7** — `fdb-realtime` gRPC client. **Conditional on OQ-FRF-1** for the live
`WatchEntityType` RPC; this change ships the production stub regardless.

## Problem
`fdb-realtime/src/lib.rs` has the tonic channel scaffolded and `watch()`
returns an empty stream with a `warn!` log. No reconnect loop, no service-
token auth, no fan-out to multiple subscriber streams. OQ-FRF-1 (FRF
`WatchEntityType` RPC) is not yet delivered upstream.

## Scope
- **Reconnect loop**: exponential backoff on stream disconnect
  (e.g., 250ms → 500ms → 1s → 5s → 30s cap) using `tokio::time` and a
  `backoff` strategy (no new dep — hand-rolled or `tokio::time::sleep` loop).
- **Service token auth**: inject the service-account bearer token into tonic
  channel headers at construction time. Token source: `flint_vault` secret
  lookup (via the privileged pool) or env var per deployment convention —
  confirm which via the spec before implementing.
- **Fan-out**: maintain `Vec<Sender<EntityChange>>` of subscriber handles;
  broadcast each received event to all subscribers (after `rls_requery`
  per G5).
- **`rls_requery()`**: implement the row re-query against the RLS pool with
  full `acquire()` GUC setup. Drop events silently when re-query returns
  zero rows. This is the non-negotiable G5 invariant (constraints.md WARN).
- **Live `WatchEntityType` call**: gated on OQ-FRF-1. If unresolved at
  apply time, keep the empty-stream placeholder with `warn!` and ship the
  scaffolding. Record a stage-gate handoff skip note explaining the deferral.

## Out of Scope
- FRF proto generation (upstream deliverable).
- Predicate-pushdown optimization (off by default per constraints.md WARN).

## Acceptance Criteria
- [ ] Reconnect loop implemented with exponential backoff + cap
- [ ] Service token auth injected into tonic channel headers
- [ ] Fan-out broadcasts events to all active subscriber streams
- [ ] `rls_requery()` uses RLS pool with full GUC setup; drops unauthorized events
- [ ] If OQ-FRF-1 unresolved: live call is empty-stream placeholder with `warn!` and documented handoff skip
- [ ] `cargo check` + clippy + `cargo test -p fdb-realtime` green
