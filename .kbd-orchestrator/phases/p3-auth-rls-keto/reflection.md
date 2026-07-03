# Reflection — p3-auth-rls-keto

_Generated: 2026-07-03. Phase status at reflection: in_progress (progress.json changes 7/9)._

> **Reconciliation note (read first).** `progress.json` tracks the original planned
> change set c010–c018 (7 `qa_passed`, c017/c018 `pending`). Substantial phase work
> landed this session as **new** changes **c019** (PostgREST query engine) and **c020**
> (LISTEN/NOTIFY change source) plus the **G4 subscription seam**, all merged to `main`
> (PRs #2, #3, #4, #5, #6). These are not yet in the `progress.json` `changes` array, so
> the 7/9 counter understates delivered work. This reflection reports against the
> **goals (G1–G7)**, which is the honest measure.

## Goal Achievement

| Goal | Status | Evidence |
| --- | --- | --- |
| **G1** — forge-policy Cedar engine | **MET** | c012 (`qa_passed`); `CedarPolicyEngine`, policies from `flint_meta.cedar_policies`. |
| **G2** — Keto coarse check (subscribe + mutation time) | **MET** | c011 KetoCheck port + c006 keto-sync cache; wired into `fdb-app` use-cases and both the mutation gate and the subscribe-time check (FabricChangeSource + ListenChangeSource). |
| **G3** — Full RLS CRUD + filter operators | **MET (exceeded)** | c013/c014 handlers; c019 replaced the ad-hoc builder with the **`fdb-query`** crate — full PostgREST parity (21 operators, logical trees, select/order/pagination/count, writes/upsert, resource embedding, FTS), one authoritative translator shared by the REST router and `PgRest`. |
| **G4** — GraphQL hybrid (Q/M passthrough + async-graphql Subscription over graphql-transport-ws + introspection merge) | **MET** | Transport + introspection merge pre-existing; the **subscription seam** (PR #2) wired the dynamic Subscription field to `Quarry::subscribe_graphql_values`; fail-closed `RlsContext` via `on_connection_init`. |
| **G5** — Subscription RLS re-query (WAL-bypass protection) | **MET** | `Quarry::subscribe_rls_filtered` (per-event re-query) merged; made live by `PgRest::execute` (c019, retired the `todo!()`); proven by the c016 mock gate + c020 live-PG test. |
| **G6** — Gate tests (4) | **MET** | c015/c016: `test_rest_select_with_eq_filter` (ported to the fdb-query bridge, all operators), `test_vault_dek_not_in_compiled_state`, `test_subscription_rls_drops_unauthorized_events`, `test_keto_check_gates_mutation`. |
| **G7** — Real-time `ChangeStreamSource` | **PARTIAL** | The named path — FRF `WatchEntityType` gRPC — remains **blocked on OQ-FRF-1** (upstream RPC not shipped); `FabricChangeSource::watch` is a documented empty-stream stub. The **capability** is delivered via an alternative: **c020 `ListenChangeSource`** (in-process Postgres LISTEN/NOTIFY), env-selectable (`FLINT_CHANGE_SOURCE=listen`), with a NOTIFY trigger migration (0006) and live-PG integration tests. |

**Phase gate:** All four auth layers (Kratos/flint-gate JWT → RLS, Keto, RLS row filter,
Cedar) are live end-to-end for REST + mutations; GraphQL subscriptions are wired and
RLS-filtered. Zero plaintext credentials in logs (enforced by the vault-DEK and
keto-subject gate tests). **Gate substantially MET**; the only residual is that the
default real-time backend (FRF) is OQ-FRF-1-blocked — the `listen` backend satisfies the
capability today.

## Delivered Changes (this session, merged to main)

- **Dev-management policy** (PR #1) — Integration-First + Compile Economy (`docs/RUST-DEVELOPMENT-MANAGEMENT.md`, CLAUDE.md, AGENTS.md, dev build profiles).
- **G4 subscription seam** (PR #2) — `SubStreamFactory`, `subscribe_graphql_values`, fail-closed `on_connection_init`; also landed the pre-existing c016 use-cases it built on.
- **c019 PostgREST query engine** (PRs #3, #4, #5) — new pure `fdb-query` crate (core + parity), `PgRest::execute` live, `fdb-reflection` REST handlers rewired, resource embedding wired into the list handler.
- **c020 LISTEN change source** (PR #6) — `fdb-realtime::ListenChangeSource`, migration 0006, gateway env-selection, live-PG integration tests.

## Artifact Quality Summary

| Metric | Value |
| --- | --- |
| Changes with artifact-refiner QA | 0 (no `.refiner/` logs; QA was CI-gate-equivalent: clippy pedantic `-D warnings` + tests) |
| Verification gate | `cargo test` + `cargo clippy --all-targets -- -D warnings` per crate, `cargo check --workspace` |
| Adversarial review | c019 parity + c020: multi-agent design→implement→adversarial-verify workflows |

**Adversarial review caught real defects (all fixed + regression-tested):**
- c019 parity: `UpdatePlan`/`DeletePlan::render` and embed `parent_alias` emitted unvalidated identifiers → now validated.
- c020: identifier-validation gap before the Keto URL, a background-task/connection leak on drop, and a subscribe-after-Keto miss window → all fixed.
- c020 live-PG run surfaced a parallel-test DDL race (`23505` / "tuple concurrently updated") → serialized with a `pg_advisory_lock` (test-only; migration was correct).

No recurring constraint-violation pattern; the recurring *lesson* is that agent-produced code passed its own crate's tests but broke a **downstream consumer** (the FTS `fts_config` field broke `Leaf` constructors in two other crates) — caught only by `cargo check --workspace`.

## Technical Debt Introduced

1. **`progress.json` drift** — c019/c020 (and the G4 seam) are merged but absent from the phase change array; c017/c018 are `pending` and now largely superseded. Needs reconciliation (see Next Phase).
2. **c017 superseded, c018 partially** — c017 (FRF reconnect stub) is superseded by c020's alternative backend; c018 (introspection-merge verify) overlaps already-merged work. Decide: archive as superseded or re-scope.
3. **Live-PG tests are `#[ignore]`-gated** — no CI Postgres, so the NOTIFY→subscription and embedding SQL paths are proven only on manual `--ignored` runs. A CI PG service would make them gating.
4. **Pre-existing, unrelated** `fdb-gateway` issues remain on `main`: a `keto_sync` env-var test flake and `uninlined_format_args` lint in `a2ui_seed_test.rs` (not introduced here; not fixed here).
5. **FRF `WatchEntityType` (OQ-FRF-1)** — still the blocking external dependency for the default real-time backend.

## Lessons Captured

- **Integration-First worked**: building the whole `fdb-query` engine + wiring both consumers before exhaustive per-unit testing surfaced the true shape; the one full-workspace `cargo check` caught the cross-crate break that isolated crate tests missed.
- **Adversarial verify is load-bearing**: every multi-agent pass found ≥1 real security/correctness defect that the implementing agent's own tests passed over.
- **Live execution beats mocks for I/O behavior**: the LISTEN live test caught a concurrency bug (DDL race) invisible to mock tests — and confirmed the migration itself was correct.
- **Never trust agent "clippy clean / tests pass" claims** — re-verified every one against the compiler; caught a `single_match_else` in my own gateway wiring and correctly attributed pre-existing failures rather than misclaiming.

## Recommended Next Phase

**First, reconcile phase bookkeeping** (a `/kbd-*` housekeeping step, not new code): register c019/c020 in `progress.json`, mark c017 superseded-by-c020 and c018 resolved/re-scoped, then the counter reflects reality and the phase can close cleanly.

**Then, candidate next-phase focus:**
1. **CI Postgres service** so the `#[ignore]`-gated live tests (embedding SQL, LISTEN→subscription) become gating — turns manual proofs into durable guarantees.
2. **OQ-FRF-1 resolution** — when FRF ships `WatchEntityType`, implement `FabricChangeSource::watch` against it; `ListenChangeSource` remains the in-process alternative.
3. **Fix the pre-existing `fdb-gateway` test debt** (env-flake isolation + the lint) as a small cleanup change.
4. Phase 4 / next epoch: Flint Kiln (`fke-*`) WASM edge-function gateway, per the roadmap.
