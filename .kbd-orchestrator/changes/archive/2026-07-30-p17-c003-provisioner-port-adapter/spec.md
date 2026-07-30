# p17-c003 — SchemaProvisioner port + PgProvisioner adapter (own pool, explicit commit)

**Phase:** p17-schema-provisioning
**Priority:** Round 2 — depends on c001 (ledger/role), c002 (types)
**Scope:** `crates/fdb-ports/src/` (new provision module), `crates/fdb-postgres/src/` (new provisioner module)
**Source:** FFS-001 D3/D7, §7 port sketch, §8 Phase 2

## Problem

DDL execution needs a seam that (a) never reuses `DatabaseBackend::acquire` —
whose BEGIN-for-connection-lifetime discipline silently discards uncommitted
writes (`crates/fdb-postgres/src/conn.rs` documents this exactly) — and
(b) runs as `flint_provisioner`, not as the caller's role and not as the
migration owner (D3 blast-radius split).

## What to build

**`fdb-ports`**: `SchemaProvisioner` trait per the FFS-001 §7 sketch
(async_trait, Send+Sync): `introspect_namespace(&Namespace) -> Vec<TableMeta>`,
`apply(&ValidatedPlan) -> AppliedPlan`, plus plan-store methods
(`persist_planned`, `load_planned(&PlanHash)`) since D-P1 makes the ledger the
plan store. Errors #[non_exhaustive], SQLSTATE-only in messages — never the
rendered statement.

**`fdb-postgres`**: `PgProvisioner`:
- OWN deadpool built from `PROVISIONER_DATABASE_URL`. Never touches the
  backend/engine pools.
- Apply: explicit `BEGIN` → generated statements in order → `COMMIT`; on any
  statement error: `ROLLBACK`, ledger row → status='failed' with SQLSTATE.
- Ledger writes: planned → applied|failed; `applied_by: &str` parameter (JWT
  sub, supplied by the gateway — adapter never sees a bearer), applied_at,
  version_before/version_after (schema version from flint_meta).
- Tracing span at the port boundary: plan_id, namespace, role — NOTHING else
  (constraints BLOCK logging claims/tenant ids/tokens).

## Constraints
- Hexagonal: fdb-postgres implements the fdb-ports trait; only fdb-gateway
  composes them.
- Tests WRITTEN here, RUN at boundary: D7 regression (fresh connection sees
  the table after apply), failure rollback (no partial table + failed ledger
  row), privilege containment (create attempt outside a granted namespace
  fails with insufficient_privilege at the Postgres layer).
