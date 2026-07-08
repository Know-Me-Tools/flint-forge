# Reflection — p14-v1.1.0

**Phase:** 14 — v1.1.0 Feature Cycle
**Period:** 2026-07-07
**Author:** OpenCode / KBD automated reflection
**Changes:** 5/5 done (1 partial — sqlx 0.9 deferred)
**Status:** ✅ COMPLETE

---

## Summary

Phase 14 delivered the first feature cycle after v1.0.0. Five changes were
planned; all reached a working, merged state. The P0 sqlx-prometheus change
was split: pool metrics were implemented on the existing sqlx 0.8 stack so the
Grafana DB connections panel and `HighDbConnections` alert can fire on real
data, while the sqlx 0.9 upgrade required to unblock `generic-array` in
`cargo update` was deferred to a future phase. The remaining P1/P2 changes —
Kiln guest SDK repair, A2UI hot-reload verification, staging JWT rotation
automation, and Kiln per-function metrics — all landed and pass CI.

---

## Goal Achievement

| Goal | Priority | Status | Evidence |
|---|---|---|---|
| G1 — sqlx Prometheus integration | P0 | **PARTIAL** | Pool metrics (`sqlx_pool_*`) added on sqlx 0.8; sqlx 0.9 upgrade deferred |
| G2 — Kiln guest Rust SDK (`flint-skill`) | P1 | **MET** | Integration test syntax and serde_json assertion fixed; cargo check/test/clippy pass |
| G3 — A2UI component hot-reload | P1 | **MET** | Migration, `broadcast_all` wiring, and SDK hook already present; verified by workspace test/clippy |
| G4 — Staging JWT rotation automation | P2 | **MET** | `scripts/rotate_staging_jwt.sh` with `--dry-run`; documented in `scripts/README.md` and `docs/runbook.md` §12 |
| G5 — Kiln per-function metrics | P2 | **MET** | `/metrics` route wired in `fke-server`; `kiln_invocations_total{function}`, `kiln_fuel_consumed_total`, `kiln_epoch_traps_total` added |

**Phase exit condition:** All planned changes implemented and workspace green → p14 complete.

---

## Deliverables

| File | Lines / Change | Purpose |
|---|---|---|
| `scripts/rotate_staging_jwt.sh` | new | Regenerates staging JWT secret and updates `STAGING_JWT_SECRET` via `gh secret set` |
| `scripts/README.md` | +section | Documents `rotate_staging_jwt.sh` |
| `docs/runbook.md` | +§12 | Staging JWT secret rotation procedure |
| `crates/fke-server/Cargo.toml` | +2 deps | `axum-prometheus`, `metrics` |
| `crates/fke-server/src/main.rs` | +metrics layer / route | `PrometheusMetricLayer`, `GET /metrics`, `kiln_invocations_total{function}` counter |
| `crates/fke-runtime/Cargo.toml` | +1 dep | `metrics` |
| `crates/fke-runtime/src/lib.rs` | +telemetry helpers | `KilnHandleOutcome`, `handle_with_telemetry()`, `is_epoch_trap()`, fuel/epoch-trap counters |

---

## Technical Debt & Carry-Forward

| Item | Source | Severity | Resolution |
|---|---|---|---|
| sqlx 0.9 upgrade | p14-c001 | MEDIUM | Required to unblock `generic-array` in `cargo update`; carry forward to p15 or dependency-maintenance phase |
| Grafana DB connections panel validation | p14-c001 | LOW | Pool metrics are emitted; panel data should be verified in a live environment |
| `flint-skill` `wasm32-wasip2` target compile | p14-c002 | LOW | Crate compiles for host target; cross-compile target availability to be verified |
| A2UI hot-reload integration test | p14-c003 | LOW | Plumbing verified by unit/workspace tests; full end-to-end file-change test not written |

**No new architectural debt introduced in p14.** The sqlx 0.9 deferral is a
conscious scope decision to keep the v1.1.0 cycle shippable.

---

## What Was Harder Than Expected

1. **Splitting sqlx-prometheus work across two sqlx versions** — The original
   goal assumed upgrading to sqlx 0.9 first. Discovery in p13 showed that
   `generic-array 0.14.9` pulls sqlx 0.9 into pgvector's dependency tree,
   breaking `Encode`/`Type` resolution. Implementing pool metrics on sqlx 0.8
   delivered value immediately while isolating the riskier upgrade.

2. **Kiln telemetry split across crates** — Invocation labels (`function` name)
   are available in `fke-server`, while fuel consumption and epoch-trap
   detection require access to the wasmtime `Store` and error inside
   `fke-runtime`. The counters were placed by data locality, with
   `handle_with_telemetry()` returning a structured outcome so the server can
   still record the labeled invocation counter and the runtime can record
   resource counters.

3. **Detecting wasmtime epoch traps** — wasmtime does not expose a stable
   typed error for epoch deadline traps. The implementation inspects
   `Error::to_string()` for the substring `"epoch"`, which is pragmatic but
   brittle if wasmtime changes its diagnostic text. A future improvement is to
   use `Store::get_epoch_deadline()` or a dedicated trap code when available.

---

## Lessons Captured

1. **Defer risky dependency upgrades without blocking user-visible value** —
   Pool metrics did not require sqlx 0.9. Shipping them on 0.8 unblocks
   monitoring while preserving the option to upgrade later.

2. **Place metrics where the data is authoritative** — Splitting counters
   across `fke-server` and `fke-runtime` is cleaner than threading function
   names into the runtime or exposing wasmtime details into the server. A
   small telemetry outcome struct keeps the boundary clean.

3. **Shell scripts for operator workflows need dry-run and documentation** —
   `rotate_staging_jwt.sh` includes `--dry-run`, `bash -n` validation, and
   runbook documentation, making it safe for on-call use.

---

## Recommended Next Phase

**Standing continuous-operations mode** or **p15-v1.2.0**, depending on
roadmap priority. Carry-forward items from p14 that could seed a next phase:

| Priority | Item | Scope |
|---|---|---|
| P0 | sqlx 0.9 upgrade + `generic-array` unblock | Medium — dependency tree reconciliation |
| P1 | End-to-end A2UI hot-reload integration test | Small — file-change trigger → SSE → client refresh |
| P1 | `flint-skill` wasm32-wasip2 compile gate | Small — toolchain/target verification |
| P2 | Grafana DB connections panel validation | Small — operator check in live env |
| P2 | k6 baselines (from p13) | Medium — blocked on staging host availability |

**Estimated scope for p15:** 3–4 changes, 1–2 sessions if focused on the sqlx
upgrade plus one feature item.

---

*Generated by OpenCode `/kbd-reflect` — 2026-07-07*
