# Goals — p15-v1.0-production-readiness

## Phase Summary

Close the gap between "workspace compiles and unit tests pass" and "a solid
Flint Forge v1.0 that can run in production." Focus on build integrity,
operator tooling, end-to-end validation, documentation accuracy, and
production packaging — not new features.

Seeded from: User directive + `p14-v1.1.0/reflection.md`

---

## Changes (5 planned)

### P0 — Blockers for any production claim

- **p15-c001 — Anvil Extension Stabilization:**
  Make all five `ext-flint-*` / `flint_*` pgrx extensions compile and pass
  `cargo pgrx test` on a single supported toolchain.
  - Unify pgrx version and Postgres target.
  - Fix `DatumWithOid` compile error in `ext-flint-meta`.
  - Resolve workspace-inheritance misconfiguration for excluded crates.
  - Add pgrx CI job in a Linux container.
  - Gate: `cargo pgrx test` passes for all extensions in CI.

- **p15-c002 — Migration Integrity:**
  Restore strict linear migration ordering and verify migrations in CI.
  - Renumber colliding `migrations/0005_*` and `migrations/0006_*` files.
  - Add CI step that runs `sqlx migrate run` against an empty Postgres 18 DB.
  - Exercise `fdb-gateway` startup migration path in an integration test.
  - Gate: `sqlx migrate run` succeeds in CI.

### P1 — Operator tooling and validation

- **p15-c003 — Operator CLI:**
  Implement `forge-cli` subcommands: `fn register`, `hook add`, `migrate`,
  `token mint`.
  - Add CLI tests and CI integration.
  - Gate: `forge --help` lists working subcommands; tests pass.

- **p15-c004 — E2E + Performance Validation:**
  Automate integration tests and establish load baselines.
  - Run `DATABASE_URL`-gated integration tests in CI with a Postgres service.
  - Add `sqlx_pool_connections_open` metric.
  - Run k6 baselines and populate `docs/performance.md`.
  - Gate: CI integration tests pass; performance thresholds committed.

### P2 — Documentation and production packaging

- **p15-c005 — Docs + Production Artifacts:**
  - Update `README.md` status and remove stale scaffold language.
  - Refresh `docs/security-audit.md` and `docs/ROADMAP.md`.
  - Add minimal Kubernetes manifests or a Helm chart.
  - Fix hard-coded Kiln cache miss in `fke-server/src/main.rs`.
  - Gate: Docs consistent; Helm chart lints; Kiln cache hit path tested.

---

## Phase Complete When

- [ ] All five Anvil pgrx extensions compile and pass `cargo pgrx test` in CI.
- [ ] `sqlx migrate run` succeeds against an empty Postgres 18 database.
- [ ] `forge-cli` implements and tests all documented subcommands.
- [ ] CI runs integration tests against a real Postgres instance.
- [ ] k6 performance baselines are committed and thresholds are real numbers.
- [ ] README, security-audit, and ROADMAP docs are internally consistent.
- [ ] A Helm chart or Kubernetes manifest set is present and lints cleanly.
- [ ] `cargo test --workspace` and `cargo clippy --workspace -- -D warnings` remain green.

---

## Dependencies

- Postgres 18 toolchain and Docker image.
- Staging host (preferred) or container-local stack for k6 baselines.
- User decision on OQ-P15-1 (pgrx target version) and OQ-P15-4 (k8s/Helm requirement).

---

## Risk areas

- **pgrx toolchain reconciliation** (c001): largest unknown; may reveal more API
  changes as each extension is fixed.
- **Migration renumbering** (c002): must not break existing deployed databases;
  may require operator coordination.
- **Integration test external dependencies** (c004): FRF/Keto may need
  testcontainers or conditional gating.
