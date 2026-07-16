# p16-c002 — Realtime must fail closed, not silently empty

**Phase:** p16-v1.0-release-closure
**Priority:** P0 — Tier 0, ship-blocker
**Scope:** `crates/fdb-realtime/src/lib.rs`, `crates/fdb-gateway/src/main.rs`,
`.env.example`, `docker-compose.yml`
**Delivery model:** model-independent (required for all)

---

## Problem

`FabricChangeSource::watch()` (`crates/fdb-realtime/src/lib.rs:116`) returns
**success with no data**:

```rust
tracing::warn!(..., "OQ-FRF-1: WatchEntityType not yet available in FRF; returning empty stream");
let empty: BoxStream<'static, Result<ChangeEvent, StreamError>> =
    futures::stream::empty().boxed();
Ok(empty)
```

A subscriber connects, the Keto coarse check passes, the GraphQL subscription
is established — and then **no event ever arrives**. The failure is invisible to
the client. It fails *open, into silence*.

`FabricChangeSource` is the **default**. `main.rs:601`:

```rust
let use_listen = std::env::var("FLINT_CHANGE_SOURCE").as_deref() == Ok("listen");
let change_source = if use_listen { ListenChangeSource::new(..) }
                    else { FabricChangeSource::new(..) };   // ← default
```

`ListenChangeSource` (`crates/fdb-realtime/src/listen.rs:215`) is a **complete,
working** LISTEN/NOTIFY implementation whose Keto check fails closed.

## Who is affected

| Deployment path | `FLINT_CHANGE_SOURCE` | Subscriptions |
|---|---|---|
| Helm (`deploy/helm/flint-forge/values.yaml:55`) | `"listen"` | ✅ work |
| `.env.example:59` | commented out | ❌ silent no-op |
| `docker-compose` | unset | ❌ silent no-op |
| `cargo run -p fdb-gateway` | unset | ❌ silent no-op |

Every quickstart path is broken. `README.md:40` advertises GraphQL subscriptions
with no caveat; `README.md:104` promises RLS enforcement on "every query /
subscription event."

For a **self-hosted OSS** product this is the worst class of defect: the
operator has no vendor to ask, and the system reports success while doing
nothing. A missing feature is a bug. A feature that pretends to work is an
incident.

## Change

Two independent corrections; **both** are required.

### 1. Fail closed (the correctness fix)

`FabricChangeSource::watch()` must return `Err(StreamError::Unavailable)`
instead of `Ok(empty_stream)` while OQ-FRF-1 is unresolved.
`StreamError::Unavailable` already exists (`crates/fdb-ports/src/lib.rs:31`) —
no new variant needed. `ListenChangeSource` already uses this convention for
Keto unavailability, so this makes the two adapters consistent.

Keep the `tracing::warn!`. Keep the commented-out future implementation.

### 2. Default to the source that works (the ergonomics fix)

Invert the default in `main.rs:601` so `ListenChangeSource` is selected unless
`FLINT_CHANGE_SOURCE=fabric` is explicitly set. Rationale: the LISTEN/NOTIFY
adapter is complete and correct; the FRF adapter is a stub blocked on an
external dependency open since p3. Defaulting to the broken one is indefensible.

Then `.env.example` and `docker-compose.yml` inherit a working default, and Helm
(which already pins `listen`) is unaffected.

### 3. Correct the misleading comment

`.env.example:58` reads: *"Set to 'listen' to use PostgreSQL LISTEN/NOTIFY
instead of polling."* There is no polling. The alternative is an empty stream.
Rewrite it to describe reality.

## Acceptance Criteria

1. With `FLINT_CHANGE_SOURCE` **unset**, a GraphQL subscription against a table
   receives an event when a row is inserted. (Today: receives nothing.)
2. With `FLINT_CHANGE_SOURCE=fabric` explicitly set, opening a subscription
   returns a **transport-level error** to the client, not an empty stream —
   the client can distinguish "unavailable" from "no events yet".
3. `deploy/helm/flint-forge/values.yaml` continues to work unchanged.
4. No `Ok(futures::stream::empty())` remains on any `ChangeStreamSource::watch`
   implementation.
5. `.env.example` no longer references "polling".

## Non-Goals

- Resolving OQ-FRF-1 / implementing `WatchEntityType`. That is an external
  dependency on the `flint-realtime-fabric` team, open since p3, with no
  resolution date. This change makes its absence **loud** rather than silent.
- Removing `FabricChangeSource`. It stays, as the typed connection boundary and
  the seam for the future RPC.

## Verification Command

```bash
# 1. unit: fabric adapter fails closed
cargo test -p fdb-realtime fabric_watch_returns_unavailable

# 2. integration (needs DATABASE_URL): default path delivers events
cargo test -p fdb-gateway --test subscriptions
```

Criterion 1 is unit-testable today. Criterion 2 depends on p16-c003 (a working
integration environment) — sequence this change **after** c001/c003 land, or
verify manually against a local Postgres.

## Risk

**Medium.** Inverting a default changes behavior for any operator who relies on
the current (broken) default — but that operator is by definition receiving no
events, so nothing that works today can break. The real risk is criterion 2's
test requiring DB infrastructure that does not yet run green in CI.

**Breaking change:** yes, per Base Rule #16. An operator explicitly depending on
`FabricChangeSource` now receives an error where they previously received
silence. Document in `CHANGELOG.md` under a `BREAKING` heading. This is a fix,
not a regression: the prior behavior was never correct.

## Open Questions

- **Does the beta include realtime subscriptions at all?** (from `analysis.md`)
  If the answer is no, the honest alternative is to remove the feature from
  `README.md` rather than ship it. This spec assumes **yes**, because the README
  advertises it and `ListenChangeSource` makes it deliverable.
- Should `FLINT_CHANGE_SOURCE=fabric` be rejected at startup (fail fast) rather
  than at subscribe time? Arguably yes — an operator setting it gets a clear
  boot error instead of a runtime surprise. Deferred to the implementer.
