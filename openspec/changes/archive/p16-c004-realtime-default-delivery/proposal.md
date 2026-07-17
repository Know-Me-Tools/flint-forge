# p16-c004 — Realtime Delivery By Default

**Phase:** 16 — Production Remediation
**Priority:** P0 (blocks any production claim)
**Depends on:** none

## What this change delivers

- GraphQL subscriptions deliver real events on the **default** configuration
  (no env var required), or the server fails loudly at startup instead of
  silently accepting subscriptions that never deliver anything.
- `docker-compose*.yml` sets the same working change-source configuration the
  Helm chart already uses.

## Problem

`FabricChangeSource::watch()` (`crates/fdb-realtime/src/lib.rs:116-126`,
OQ-FRF-1) performs the Keto check correctly, then returns
`futures::stream::empty()` with a `tracing::warn!` — because the upstream FRF
`WatchEntityType` RPC doesn't exist yet. `fabric` is the **default** backend
selected in `crates/fdb-gateway/src/main.rs:601-620` unless
`FLINT_CHANGE_SOURCE=listen` is set. The alternative `ListenChangeSource`
(Postgres LISTEN/NOTIFY) is real and working — it's what `deploy/helm/flint-forge/values.yaml:55`
already sets. `docker-compose.yml` / `docker-compose.prod.yml` / `docker-compose.staging.yml`
set none of this, so a subscription in a compose deployment authenticates,
opens, and **silently delivers nothing**, indistinguishable from "no changes
happened."

## Design

**Decision: (a).** No channel exists to actually "check with the FRF team" in
this environment, so the decision is made from the evidence available in the
repository itself: `crates/fdb-realtime/src/lib.rs`'s own comments (`OQ-FRF-1`,
`:116-126`) confirm `WatchEntityType` has not landed — `stream::empty()` is
still the live behavior, with no indication anywhere in the codebase that this
has changed. Given the alternative (`ListenChangeSource`) is real, tested, and
already the Helm chart's default, flipping to it is the safe, immediately
actionable choice that doesn't depend on an unresolved external dependency.
(b) remains available as a fallback if FRF's RPC lands before this executes
and the fabric default is deliberately kept — re-evaluate then, not now.

- Flip the default in `main.rs:601` from `fabric` to `listen`. Keep
  `FLINT_CHANGE_SOURCE=fabric` as an explicit opt-in once FRF is ready. Update
  the doc comment at `main.rs:598-600` accordingly.
- Add `FLINT_CHANGE_SOURCE=listen` to `docker-compose.yml`,
  `docker-compose.prod.yml`, and `docker-compose.staging.yml` so compose
  deployments match Helm's already-correct behavior.

## Verification (gate)

- Integration test (extend `crates/fdb-realtime/tests/listen_live_pg.rs`
  pattern, or add a GraphQL-subscription-level test): a subscription opened on
  the **default** configuration receives a real `ChangeEvent` after a row
  mutation — no env var override needed in the test setup.
- Confirm no code path returns `stream::empty()` silently without either
  delivering events or surfacing a `StreamError` the client can observe.
