# Flint Forge — Roadmap

This document captures prioritised items for the next development cycle (`v1.1.0`).
When this list reaches 3+ actionable items, a new KBD phase (`p14-*`) is opened.

Items are tagged: `breaking` / `additive` / `fix` / `ops`.

---

## Status

| Cycle | Status | Items |
|---|---|---|
| v1.0.0 (p12) | ✅ Released | — |
| v1.1.0 | 🟡 Planning | 6 items below |

---

## Prioritised items for v1.1.0

### P0 — Carry-forward debt

**[ops] k6 performance baselines** (carry from p13-c001)
Measure real P50/P95/P99 against a live staging stack; update `regression.js`
and `docs/performance.md`. Blocked on a cloud host being provisioned.
*Scope: small — one staging run.*

---

### P1 — High-value additions

**[additive] sqlx Prometheus integration**
Emit `sqlx_pool_connections_open`, `sqlx_pool_connections_idle`, and
`sqlx_pool_acquire_total` counters so the Grafana DB connections panel
and `HighDbConnections` alert produce real data. Requires either a
custom `sqlx` pool listener or a crate that wraps the pool with metrics.
*Scope: medium — new crate dependency or custom pool wrapper.*

**[additive] Kiln guest Rust SDK (`flint-skill`)**
A Rust helper crate targeting `flint:host@0.1.0` that provides ergonomic
wrappers around the raw WIT bindings: typed `db::query`, `llm::complete`,
`kv::get`/`set`, `identity::claims`. Lowers the bar for skill authors and
eliminates boilerplate.
*Scope: medium — new crate, no changes to the host ABI.*

**[additive] A2UI component hot-reload**
When `flint-gate` or an operator pushes a DESIGN.md update, the StateManager
should re-compile the A2UI component catalog and notify connected AG-UI SSE
clients without a service restart. Extends the existing hot-reload path.
*Scope: medium — extends the existing `StateManager::do_compile` path.*

---

### P2 — Operational improvements

**[ops] STAGING_JWT_SECRET rotation automation**
`scripts/mint_smoke_token.sh` currently requires the operator to manually
set `STAGING_JWT_SECRET` in the GitHub Actions environment. Automate the
rotation: add a `scripts/rotate_staging_jwt.sh` that re-generates
`secrets/jwt_secret.txt` and updates the GitHub Actions secret via `gh secret set`.
*Scope: small — 30-line shell script.*

**[additive] Observability: per-route Kiln invocation metrics**
Currently `fke-server` emits only the standard `axum_http_requests_total`
histogram. Add Kiln-specific counters: `kiln_invocations_total` (by function
name), `kiln_fuel_consumed_total`, and `kiln_epoch_traps_total`. Enables
function-level performance analysis.
*Scope: small — 3 counter registrations in `fke-server/src/main.rs`.*

---

## Trigger for p14

When 3+ items from the P1/P2 list above are approved for implementation,
open the next phase:

```bash
/kbd-new-phase p14-v1.1.0
```

The P0 carry-forward item (k6 baselines) can be done at any time without a
new phase — it is an operator action, not a code change.

---

## Not-in-scope (deferred indefinitely)

| Item | Reason |
|---|---|
| Grafana DB connections panel | Blocked on sqlx Prometheus integration (listed above as P1) |
| Multi-tenant A2UI isolation | Requires Cedar policy redesign; not yet prioritised |
| `cargo nextest` in CI | Ergonomic improvement only; existing `cargo test` gate is sufficient |
