# p0-c004 (cross-repo: flint-realtime-fabric) — WatchEntityType RPC

## Why
Quarry subscriptions (Phase 3) need a server-streaming RPC that filters CDC by tenant +
entity_type at the fabric. This is the single cross-repo change; it gates Phase 3 only.

## What (in flint-realtime-fabric)
- `proto/flint/v1/entity.proto`: add `WatchEntityType(WatchEntityTypeRequest) returns (stream EntityChange)`
  with `{ tenant_id, entity_type, filter }`.
- `frf-gateway` SubscribePipeline: entity-type filter path + Keto coarse gate.
- Keep existing single-entity `WatchEntity` intact.

## Contract
A client streaming `WatchEntityType(tenant, entity_type)` receives `EntityChange` events for
matching rows; Keto denies unauthorized subject/entity-type pairs at subscribe time.
