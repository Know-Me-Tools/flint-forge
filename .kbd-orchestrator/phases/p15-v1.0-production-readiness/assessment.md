# Assessment — p15-v1.0-production-readiness

**Phase:** 15 — v1.0 Production Readiness Gap Closure
**Assessment date:** 2026-07-07
**Author:** OpenCode / KBD assessment
**Seeded from:** User directive to close out Flint Forge as a solid v1.0 with sufficient tools, CLIs, documentation, and SDKs.

---

## Goal Restatement

Deliver a production-credible Flint Forge v1.0: the core server plane, pgrx
extension suite (Anvil), operator CLI, client SDKs, documentation, and
operational artifacts must all be buildable, tested, internally consistent,
and deployable.

This is **not** a feature phase. It is a stabilization and completeness phase
driven by the gap between "workspace compiles and unit tests pass" and "a first
version you can run in production."

---

## Current State vs. v1.0 Goal

### What already works (evidence)

- **Core server plane is substantially implemented.** `fdb-gateway`
  (`crates/fdb-gateway/src/main.rs`) and `fke-server`
  (`crates/fke-server/src/main.rs`) are fully wired, not scaffold stubs.
- **Workspace builds and tests green.** `cargo check --workspace` and
  `cargo test --workspace --lib --bins` pass with 470+ tests.
- **Zero executable `todo!()` stubs.** `rg '\btodo!\(\)' crates/` finds only
  doc-comment/test-comment occurrences (`crates/fdb-reflection/src/compilers/rest/mod.rs:1`,
  `crates/fdb-realtime/src/lib.rs:1`, `crates/fdb-gateway/tests/mounts_reflection_router.rs:5`).
- **CI is meaningful.** `.github/workflows/ci.yml` runs fmt, clippy `-D warnings`,
  test, `cargo audit`, API version check, and `cargo component build`.
- **Real feature coverage:** REST CRUD with PostgREST-style filters and resource
  embedding, GraphQL query/mutation passthrough + async-graphql subscriptions,
  A2UI registry + protocol surfaces, MCP/A2A/AG-UI endpoints, HTMX rendering,
  Keto/Cedar authorization, rate limiting, JWT/RLS context, Kiln WASM component
  runtime with signing/capability gating, and Prometheus `/metrics`.
- **SDKs exist.** `crates/flint-skill` (Rust guest SDK for Kiln skills),
  `packages/flint-react` (React component library + AG-UI + registry hooks),
  `packages/flint_genui` (Flutter SDK).
- **Docker Compose overlays** for dev, staging, and prod exist with Caddy,
  Prometheus, Alertmanager, and Docker secrets.
- **Observability stack** is present: `/metrics`, structured tracing with OTLP,
  Grafana dashboard, alert rules (`observability/alerts.rules.yml`).

### What does not yet meet v1.0 bar

| # | Gap | Severity | Evidence |
|---|---|---|---|
| 1 | **Anvil pgrx extensions do not build together.** | P0 / blocker | `ext-flint-meta` fails to compile (`DatumWithOid` undefined, `crates/ext-flint-meta/src/functions.rs:42,70`). `ext-flint-auth`/`ext-flint-hooks` inherit workspace settings while being excluded from the workspace → `cargo pgrx` workspace-root errors. `flint_vault`/`flint_llm` use pgrx 0.18.1 and fail macOS linking with undefined Postgres symbols. |
| 2 | **Migration sequence collisions.** | P0 / blocker | `migrations/0005_cedar_policies.sql` and `migrations/0005_flint_a2ui_hybrid_search.sql` share `0005`; same for `0006_change_notify.sql` and `migrations/0006_flint_a2ui_application_model.sql`. `sqlx migrate run` aborts on duplicates. |
| 3 | **Operator CLI is a stub.** | P1 | `crates/forge-cli/src/main.rs` only prints `version`; `fn register`, `hook add`, `migrate` are listed but TODO. |
| 4 | **pgrx version / toolchain inconsistency.** | P1 | `ext-flint-auth`/`ext-flint-hooks` pin pgrx `0.12` (pg17); `flint_vault`/`flint_llm`/`ext-flint-meta` pin `=0.18.1` (pg18). Workspace `rust-version = "1.85"` conflicts with pgrx 0.18.1's documented Rust requirement (`>= 1.96`). |
| 5 | **No end-to-end test coverage in CI.** | P1 | Integration tests requiring `DATABASE_URL` are not exercised in CI. k6 scripts in `perf/k6/` are manual-only. |
| 6 | **Performance baselines are placeholders.** | P2 | `docs/performance.md` and `perf/k6/regression.js` contain "TBD" thresholds; no measured P50/P95/P99. |
| 7 | **Documentation is internally inconsistent.** | P2 | `README.md` still says "Status: scaffold" and references `todo!()` stubs. `docs/security-audit.md` claims `cargo audit` is not in CI, but it is. `docs/ROADMAP.md` lists `flint-skill` as a v1.1 item, yet it already exists. |
| 8 | **DB connection metrics not emitted.** | P2 | `docs/monitoring.md` notes the Grafana DB connections panel relies on `sqlx_pool_connections_open`, which is not produced. p14-c001 deferred the sqlx 0.9 upgrade that would unblock an upstream integration. |
| 9 | **No Kubernetes / Helm production artifacts.** | P2 | Only Docker Compose is provided. No k8s manifests, Helm charts, or operators for large-scale deployment. |
| 10 | **Kiln runtime cache miss is hard-coded.** | P3 | `crates/fke-server/src/main.rs:205` always treats the cache as a miss (`false // TODO: expose is_loaded()`), potentially reloading WASM bytes every invocation. |

### Scorecard

| Dimension | Status | Notes |
|---|---|---|
| Core server completeness | ✅ | Quarry + Kiln gateways are real implementations. |
| Build / unit tests | ✅ | `cargo check`/`cargo test --lib --bins` pass. |
| Anvil pgrx extensions | ❌ | Do not compile; version mismatch; workspace misconfiguration. |
| Migrations / boot integrity | ❌ | Sequence collisions block `sqlx migrate run`. |
| Operator CLI / tools | 🟡 | `forge-cli` is a stub; scripts exist but CLI surface incomplete. |
| SDKs | 🟡 | Rust, React, Flutter SDKs exist; not published and need validation gates. |
| Documentation | 🟡 | Broad coverage, but stale/inconsistent claims. |
| E2E / performance validation | 🟡 | Unit tests only; no CI integration tests; k6 baselines TBD. |
| Observability | 🟡 | Metrics/tracing present; DB pool metric gap. |
| Security | 🟡 | Auth/RLS/policy wired; security-audit doc stale. |
| Deployment / ops | 🟡 | Docker Compose only; no k8s/Helm. |

**Overall:** The server plane is production-credible. The extension suite and
boot-time migrations are **not**. A defensible v1.0 release requires fixing the
Anvil build and migration ordering, plus CLI/tooling/docs polish.

---

## Recommended Phase Scope

### p15-c001 — Anvil Extension Stabilization (P0)

Make the entire Anvil pgrx extension suite compile and pass `cargo pgrx test` on
a supported toolchain.

**Tasks:**
- Choose a single pgrx version + Postgres target that works across all five
  extensions and the Docker Postgres image. Likely pgrx `0.18.1` + `pg18`, with
  a Rust toolchain bump to `>= 1.96` for the extension build.
- Remove workspace inheritance from `ext-flint-auth` and `ext-flint-hooks`, or
  include all pgrx crates in a separate pgrx workspace overlay.
- Fix `DatumWithOid` usage in `ext-flint-meta/src/functions.rs` (replace with
  pgrx 0.18.1 equivalent API).
- Resolve linker configuration for `flint_vault`/`flint_llm` (macOS local dev +
  Linux CI).
- Add a pgrx CI job that runs in a Linux container with the chosen Postgres
  toolchain.

**Gate:** `cargo pgrx test` passes for all five extensions in CI.

### p15-c002 — Migration Integrity (P0)

Restore strict linear migration ordering and add CI verification.

**Tasks:**
- Renumber colliding migrations to a strict `0001..0010` sequence.
- Add a CI step that runs `sqlx migrate run` (or `sqlx migrate info`) against a
  containerized Postgres.
- Ensure `fdb-gateway` startup migration path
  (`crates/fdb-gateway/src/main.rs:95-98`) is exercised in an integration test.

**Gate:** `sqlx migrate run` succeeds against an empty Postgres 18 database in CI.

### p15-c003 — Operator CLI (P1)

Turn `forge-cli` from a stub into a usable operator tool.

**Tasks:**
- Implement `forge fn register <path>` — register a WASM component via the Kiln
  admin API.
- Implement `forge hook add <table> <url>` — add a webhook dispatch rule.
- Implement `forge migrate` — apply SQL migrations (delegate to `sqlx-cli` or
  psql with env-based connection string).
- Implement `forge token mint` (wraps existing `scripts/mint_smoke_token.sh`
  logic into the CLI).
- Add CLI tests and integrate into CI.

**Gate:** `forge --help` lists working subcommands; CLI tests pass in CI.

### p15-c004 — E2E + Performance Validation (P1)

Automate integration and load validation.

**Tasks:**
- Wire `DATABASE_URL` integration tests into CI using a Postgres service
  container or testcontainers.
- Fix or gate integration tests that require external services (FRF, Keto).
- Run k6 baselines against a staging stack and populate
  `docs/performance.md` + `perf/k6/regression.js` with measured P50/P95/P99.
- Add the `sqlx_pool_connections_open` metric (custom pool listener or sqlx 0.9
  upgrade) so the DB connections alert works.

**Gate:** CI integration tests pass; k6 regression thresholds committed.

### p15-c005 — Docs + Production Artifacts (P2)

Close consistency gaps and add production packaging.

**Tasks:**
- Update `README.md` status and remove outdated scaffold language.
- Refresh `docs/security-audit.md` and `docs/ROADMAP.md` to reflect current state.
- Add Kubernetes manifests or a Helm chart for production deployment.
- Fix the hard-coded Kiln cache miss (`crates/fke-server/src/main.rs:205`) by
  exposing `is_loaded()` on the registry/store adapter.

**Gate:** README/ROADMAP/security-audit are internally consistent; Helm chart
passes `helm lint`; Kiln cache hit path exercised in tests.

---

## Open Questions

1. **OQ-P15-1:** Which single pgrx version and Postgres version should be the
   v1.0 target? **→ Decision: pgrx 0.18.1 + pg18; bump Rust toolchain to ≥ 1.96**
   for the extension build.
2. **OQ-P15-2:** Is a staging host available to run k6 baselines, or should
   performance validation be container-local only for v1.0? **→ Decision: use the
   local Colima Docker machine for staging / k6 baselines.**
3. **OQ-P15-3:** Should `forge-cli` ship as a single static binary or as a
   containerized operator command? **→ Decision: BOTH. Provide a static binary
   and a container image; runtime mode selected by command / env.**
4. **OQ-P15-4:** Do we need a Kubernetes/Helm deliverable for v1.0, or is
   Docker Compose sufficient for the first release? **→ Decision: include a
   minimal Helm chart as part of p15-c005 (production artifacts).**

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| pgrx toolchain upgrade reveals more API changes | High | High | Scope c001 first; run `cargo pgrx test` after every extension fix. |
| Migration renumbering conflicts with deployed environments | Medium | High | Provide a manual `sqlx migrate info` check and a renumber map. |
| Integration tests need external services (FRF/Keto) | Medium | Medium | Use testcontainers or conditional compilation; fail tests gracefully. |
| k6 baselines blocked on staging host | Medium | Low | Run local container baselines and mark staging validation as follow-up. |
| Helm chart scope creep | Low | Medium | Keep v1.0 chart minimal (Deployment + Service + ConfigMap + Secret). |

---

## Estimation

| Change | Effort | Notes |
|---|---|---|
| p15-c001 Anvil stabilization | 2–3 weeks | Long pole; toolchain + API reconciliation. |
| p15-c002 Migration integrity | 2–3 days | Mechanical renumber + CI step. |
| p15-c003 Operator CLI | 4–5 days | Subcommands + tests + CI. |
| p15-c004 E2E + performance | 1–1.5 weeks | Depends on staging host for real baselines. |
| p15-c005 Docs + production artifacts | 4–5 days | Helm chart is the largest piece. |

**Total:** ~5–7 weeks of focused engineering to reach a defensible v1.0
production release.

---

## Recommended Decision

Approve **p15-v1.0-production-readiness** with the five changes above. Start
with **p15-c001 (Anvil stabilization)** and **p15-c002 (migration integrity)**
in parallel — both are P0 blockers and independent. Once they are green,
proceed to CLI, E2E/perf, and docs/artifacts.

If the goal is instead to release immediately, the minimum viable subset is
**p15-c001 + p15-c002 only**, with a known limitation that operator tooling
and k8s packaging are post-v1.0 follow-ups.

---

*Generated by OpenCode `/kbd-assess` — 2026-07-07*
