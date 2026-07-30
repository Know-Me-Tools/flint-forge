# p17-c004 — Gateway /schema/v1 routes: plan, apply, status + require_provisioner

**Phase:** p17-schema-provisioning
**Priority:** Round 3 — depends on c002, c003
**Scope:** `crates/fdb-gateway/src/routes/schema/` (new), `bootstrap.rs`, route tests
**Source:** FFS-001 §4.1–4.3, §4.5, §8 Phase 3; plan.md D-P1/D-P3/D-P4 + Unresolved Review Findings

## Problem

The API surface clients call does not exist; the auth gate, the 503
disabled-state contract, and the drift guard all live here.

## What to build

- `require_provisioner(&HeaderMap) -> Result<RlsContext, Response>` — copy the
  `require_admin` idiom from `crates/fke-server/src/handlers/admin.rs`:
  401 missing Authorization header / 401 invalid or expired token /
  403 provisioner role required (role != "service_role"). Uses
  `fdb_auth::rls_from_bearer`.
- **Always-mount, gate in handler** (plan.md Unresolved Findings #1): the
  `/schema/v1` group mounts UNCONDITIONALLY in bootstrap.rs (MCP-group idiom);
  every handler first checks the parsed FLINT_PROVISION_NAMESPACES allowlist
  and returns 503 "schema provisioning is not enabled" when empty/unset.
  FFS-001 task 3.3's "feature-gate mounting" wording is overridden in favor of
  the §4.1 response table (unmounted = 404 ≠ required 503).
- `routes/schema/{mod,plan,apply,status}.rs` split up front (500-line BLOCK).
- POST /schema/v1/plan: deserialize typed spec (closed enums) → validate
  (reserved refusal, then allowlist) → introspect_namespace → generate() →
  persist_planned (upsert by planHash; one flint_schema ledger row, zero
  user-schema writes) → 200 {planId, planHash, namespace, operations, ddl,
  warnings, noop, expiresAt=+24h}.
- POST /schema/v1/apply {planHash}: load_planned (404 unknown / 410 expired)
  → re-plan → hash compare (409 drift) → provisioner.apply with
  applied_by = caller RlsContext JWT sub (never the bearer) → 200
  {planId, applied, alreadyApplied, schemaVersionBefore/After,
  reflectionRefreshed, restartRequired:true, restartNote}. 403 non-allowlisted
  namespace; 500 + failed ledger row (SQLSTATE only) on DDL failure.
- GET /schema/v1/status: {enabled, namespaces, schemaVersion (from
  state_manager.current()), lastApply {planId, at, status} | null}. Works
  before any provisioning has occurred.
- OpenAPI documentation for the group.

## Constraints
- Composition root only: this is the sole crate knowing both port and adapter.
- Route tests WRITTEN here (run at boundary), driven by the REAL Sansaba keys,
  covering: 401 no header, 401 bad signature, 403 authenticated/anon role,
  503 disabled (pin 503-not-404), 200 plan, 200 apply, 200 alreadyApplied,
  409 drift, 403 reserved namespace.
