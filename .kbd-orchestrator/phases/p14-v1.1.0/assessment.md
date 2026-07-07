# Assessment — p14-v1.1.0

**Phase:** 14 — v1.1.0 Feature Cycle
**Assessed:** 2026-07-07
**Assessor:** OpenCode / KBD automated assess
**Changes in scope:** 5 (p14-c001 through p14-c005)
**Prior phase:** p13-continuous-operations (4/5 done, 1 deferred)

---

## Summary

Five changes from `docs/ROADMAP.md`. The P0 (sqlx Prometheus + 0.9 upgrade)
is the highest-risk item — 6 crates depend on sqlx directly, and the upgrade
must not break pgvector. The existing `StateManager` already has a LISTEN/NOTIFY
hot-recompile path (`listen_loop` → `do_compile` → `version_tx.send`), so the
A2UI hot-reload change is a notification plumbing extension, not new
architecture. `wasm32-wasip2` is installed and available for the Kiln SDK.
`fke-server` has no metrics infrastructure yet — the Kiln metrics change needs
`axum-prometheus` added to that crate.

---

## Goal-by-Goal Gap Analysis

### G1 — sqlx Prometheus Integration (`p14-c001`) — ❌ NOT STARTED

**What exists:**
- `sqlx = "0.8"` in workspace deps (6 crates use it directly)
- `pgvector 0.4.2` supports `sqlx >= 0.8, < 0.10`
- `axum-prometheus = "0.10"` in `fdb-gateway` (HTTP metrics only; no pool metrics)
- No `sqlx_pool_*` metrics emitted anywhere
- Grafana panel 4 (`HighDbConnections` alert) shows "no data"

**Two-part change:**

**Part A: sqlx 0.8 → 0.9 upgrade**
- Bump `sqlx = "0.9"` in workspace `[workspace.dependencies]`
- 6 crates pull from workspace: `fdb-app`, `fdb-gateway`, `fdb-realtime`,
  `fdb-reflection`, `fke-registry`, `fke-server`
- Verify pgvector `Encode`/`Type` traits unify across sqlx 0.9
- Run `cargo update generic-array` to confirm the transitive conflict is resolved
- **Risk:** sqlx 0.9 may have minor API changes (connection options, pool config)
- **OQ-P14-1 resolution:** Check sqlx 0.8→0.9 migration guide for breaking changes

**Part B: Pool metrics emission**
- Use `sqlx::postgres::PgPoolOptions` with a custom pool hook, or
  the `metrics` crate's `describe_gauge!`/`gauge!` macros polled on an interval
- Recommended approach: spawn a background task that reads
  `pool.size()`, `pool.num_idle()`, `pool.num_connections()` every 15s and
  emits `sqlx_pool_connections_open`, `sqlx_pool_connections_idle` gauges
- Wire into existing `telemetry::init_tracing()` or a new `pool_metrics_loop()`

**Gaps:**

| Gap | Severity |
|---|---|
| sqlx 0.8 → 0.9 version bump (6 crates) | P0 |
| No pool metrics emission infrastructure | P0 |
| `generic-array` still excluded from `cargo update` | P0 |

**Effort estimate:** Medium. The version bump is mechanical; the metrics loop is ~30 lines.

---

### G2 — Kiln Guest Rust SDK (`p14-c002`) — ❌ NOT STARTED

**What exists:**
- `wasm32-wasip2` target is **installed and available** ✅
- `wit/flint/host/world.wit` defines `flint:host@0.1.0` with 5 interfaces
- `examples/hello-component/src/bindings.rs` — auto-generated WIT bindings via `cargo component`
- No `crates/flint-skill/` crate exists

**OQ-P14-2 resolved:** `wasm32-wasip2` is installed. `cargo component build -p hello-component` already works. The new crate will use the same bindings generation path.

**Design decision:** The SDK should be a **thin ergonomic wrapper** over the
generated WIT bindings, not a replacement for them. Skill authors who want
raw access can use the bindings directly; `flint-skill` provides typed errors,
ergonomic async functions, and documentation.

**Gaps:**

| Gap | Severity |
|---|---|
| No `crates/flint-skill/` crate | P1 |
| No typed error wrappers for WIT host-error | P1 |
| No quick-start documentation for skill authors | P1 |

**Effort estimate:** Medium. New crate; ~500 lines of wrappers + tests + docs.

---

### G3 — A2UI Component Hot-Reload (`p14-c003`) — ⚠️ PARTIAL (infra exists)

**What exists:**
- `StateManager` already has a `listen_loop()` that listens on `meta_runtime`
  PostgreSQL NOTIFY channel and triggers `do_compile()` → `version_tx.send()`
- `subscribe_version()` returns a `watch::Receiver<u64>` for version changes
- AG-UI SSE broadcast infrastructure (`AgUiState` with per-run `broadcast::Sender<AgUiEvent>`)
- `@flint/react` `useFlintRegistry()` hook uses SWR for stale-while-revalidate

**What's missing:**
- The `meta_runtime` NOTIFY trigger currently fires on DDL changes (table/column add/drop).
  A2UI catalog changes (INSERT/UPDATE/DELETE on `flint_a2ui.components`) do NOT
  trigger a NOTIFY — so the StateManager doesn't re-compile when components change.
- No SSE notification from `StateManager` version change → AG-UI clients
- The `useFlintRegistry()` hook polls SWR with a default interval but doesn't
  react to a push notification

**Required changes:**
1. Add a PostgreSQL trigger on `flint_a2ui.components` (and related tables)
   that fires `pg_notify('meta_runtime', 'a2ui_change')` on INSERT/UPDATE/DELETE
2. In `fdb-gateway/src/main.rs` or `agui_hook_dispatcher.rs`: subscribe to
   `state_manager.subscribe_version()` and emit an AG-UI event on version change
3. In `@flint/react`: `useFlintRegistry()` listens for the AG-UI event and
   calls `mutate()` (SWR revalidation) on receipt

**Gaps:**

| Gap | Severity |
|---|---|
| No DB trigger for A2UI catalog changes → NOTIFY | P1 |
| No wiring from StateManager version change → AG-UI SSE | P1 |
| `useFlintRegistry()` doesn't react to push notifications | P1 |

**Effort estimate:** Medium. Migration + Rust event wiring + SDK hook update.

---

### G4 — JWT Rotation Automation (`p14-c004`) — ❌ NOT STARTED

**What exists:**
- `scripts/mint_smoke_token.sh` generates 1-hour JWTs
- `scripts/rotate_secrets.sh` regenerates `secrets/jwt_secret.txt`
- `gh` CLI v2.95.0 installed and authenticated
- `STAGING_JWT_SECRET` documented in runbook §9.1

**Gaps:**

| Gap | Severity |
|---|---|
| No `scripts/rotate_staging_jwt.sh` | P2 |
| Manual `gh secret set` step not automated | P2 |

**Effort estimate:** Small. ~40-line shell script.

---

### G5 — Kiln Per-Function Metrics (`p14-c005`) — ❌ NOT STARTED

**What exists:**
- `fke-server` has NO metrics infrastructure (no `axum-prometheus`, no `/metrics` endpoint)
- `fdb-gateway` has the full `axum-prometheus` + `/metrics` setup from p9-c004

**Gaps:**

| Gap | Severity |
|---|---|
| `fke-server` has no `axum-prometheus` dependency | P2 |
| `fke-server` has no `/metrics` endpoint | P2 |
| No `kiln_invocations_total` / `kiln_fuel_consumed_total` / `kiln_epoch_traps_total` counters | P2 |

**Effort estimate:** Small–medium. Add `axum-prometheus` to `fke-server` (same
pattern as `fdb-gateway`'s `telemetry.rs`), then add 3 custom counters in
`invoke_impl()`. ~80 lines of code.

---

## Open Questions — Resolution

| OQ | Resolution |
|---|---|
| OQ-P14-1: sqlx 0.9 API surface vs 0.8 | **Needs verification.** sqlx 0.9 is a minor bump; likely no breaking changes for our usage (`PgPool`, `query_as`, `migrate!`). The main risk is `PgPoolOptions` API. Check during c001 implementation. |
| OQ-P14-2: wasm32-wasip2 target | **Available.** `rustup target list` shows `wasm32-wasip2 (installed)`. `cargo component build -p hello-component` already uses it. |

---

## Priority Stack for Planning

```
P0 — Must ship:
  1. p14-c001-sqlx-prometheus    — sqlx 0.9 upgrade + pool metrics
                                    (unblocks cargo update; fixes Grafana panel)

P1 — Should ship (independent of each other):
  2. p14-c002-kiln-guest-sdk     — new crate, no deps on others
  3. p14-c003-a2ui-hot-reload    — migration + event wiring + SDK hook

P2 — Ship if capacity allows:
  4. p14-c005-kiln-metrics       — add axum-prometheus to fke-server + 3 counters
  5. p14-c004-jwt-rotation-automation — shell script + gh secret set
```

**c001 must run first** — the sqlx 0.9 upgrade changes the Cargo.lock baseline
for all other changes. c002 and c003 are independent and can run in parallel
after c001. c004 and c005 are small and can be batched.

---

## MVP Gate — Current Status

| Gate condition | Current state | Gap |
|---|---|---|
| sqlx pool metrics on `/metrics` | ❌ no pool metrics | c001 |
| `cargo update` succeeds without pgvector break | ❌ `generic-array` excluded | c001 |
| `flint-skill` compiles for `wasm32-wasip2` | ❌ crate absent | c002 |
| A2UI hot-reload triggers re-compile + SSE | ⚠️ infra exists; trigger + wiring missing | c003 |
| `cargo test --workspace` passes | ✅ 457 tests | — |
| `cargo clippy --workspace -- -D warnings` clean | ✅ | — |

**Two of six gate conditions pass.** Four require p14 changes.

---

*Assessment complete. Proceed to `/kbd-plan p14-v1.1.0`.*
