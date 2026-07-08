# Flint Forge — Roadmap

This document captures prioritised items for the current and next development cycles.
When the `v1.1.0` list reaches 3+ actionable items, a new KBD phase is opened.

Items are tagged: `breaking` / `additive` / `fix` / `ops`.

---

## Status

| Cycle | Status | Items |
|---|---|---|
| v1.0.0 (p15) | ✅ Released | Anvil stabilization, migration integrity, operator CLI, E2E/perf validation, Helm chart |
| v1.1.0 | 🟡 Planning | 4 items below |

---

## Completed in v1.0.0 (p15)

The following items from the previous roadmap were delivered as part of the
v1.0 production-readiness phase:

| Item | Status | Where |
|---|---|---|
| k6 performance baselines | ✅ Done | `perf/k6/regression.js`, `docs/performance.md` (local Colima baseline) |
| sqlx Prometheus integration | ✅ Done | `crates/fdb-gateway/src/telemetry.rs` — `sqlx_pool_connections_open` / `_idle` |
| Kiln guest Rust SDK (`flint-skill`) | ✅ Done | `crates/flint-skill` |
| A2UI component hot-reload | ✅ Done | `fdb-gateway/src/main.rs` StateManager version watcher + AG-UI broadcast |
| STAGING_JWT_SECRET rotation automation | ✅ Done | `scripts/rotate_staging_jwt.sh` |
| Per-route Kiln invocation metrics | ✅ Done | `fke-server/src/main.rs` — `kiln_invocations_total`, `kiln_fuel_consumed_total`, `kiln_epoch_traps_total` |
| Grafana DB connections panel data | ✅ Done | Driven by `sqlx_pool_connections_open` |

---

## Prioritised items for v1.1.0

### P1 — High-value additions

**[additive] Cloud k6 baselines and SLO dashboard**
Re-run `perf/k6/regression.js` against a production-like staging host, commit
production P50/P95/P99 thresholds, and add a Grafana SLO dashboard.
*Scope: small — operator action plus dashboard JSON.*

**[additive] Publish SDK packages**
Publish `packages/flint-react` and `packages/flint_genui` to a registry, and
`crates/flint-skill` / `forge-cli` to crates.io with versioned release tags.
*Scope: medium — packaging, docs, and release automation.*

**[breaking] sqlx 0.9 upgrade**
Upgrade the workspace to `sqlx` 0.9 once pgrx toolchain work unblocks it, enabling
the upstream pool metrics listener and query logging improvements.
*Scope: medium — API changes and migration verification.*

---

### P2 — Operational improvements

**[ops] `cargo deny` policy gate**
Combine license, advisory, and crate ban enforcement in CI alongside the existing
`cargo audit` step.
*Scope: small — new config file + CI step.*

---

## Trigger for next phase

When 3+ items from the P1/P2 list above are approved for implementation, open
a new v1.1.0 phase:

```bash
/kbd-new-phase p16-v1.1.0-enhancements
```

---

## Not-in-scope (deferred)

| Item | Reason |
|---|---|
| Multi-tenant A2UI isolation | Requires Cedar policy redesign; not yet prioritised |
| `cargo nextest` in CI | Ergonomic improvement only; existing `cargo test` gate is sufficient |
| Kubernetes operator | Out of scope for v1.x; Helm chart covers deployment needs |
