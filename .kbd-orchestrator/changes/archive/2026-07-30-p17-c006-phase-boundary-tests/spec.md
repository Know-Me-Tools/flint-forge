# p17-c006 — Phase-boundary test run + end-to-end suite (Integration-First checkpoint)

**Phase:** p17-schema-provisioning
**Priority:** Round 5 — depends on c001–c005 all implementation-complete
**Scope:** CI config, `crates/fdb-gateway/tests/`, run records
**Source:** FFS-001 §9; AGENTS.md Integration-First (binding); plan.md D-P5/D-P6

## Problem

Per AGENTS.md, the suite runs ONCE when the phase is implementation-complete
(no todo!() on live paths, no port without adapter, no unmounted handler) —
this change IS that checkpoint, plus the two tests that only exist end-to-end.

## What to build

1. FIRST (before any run): reconcile the 90% changed-crate coverage gate
   (p16-c009) with DATABASE_URL-gated integration tests. Determine whether CI
   coverage measurement counts them; if not, either wire the gate to include
   the gated tests in a Postgres-service CI job, or record a documented
   concession in this change's verification.md. Do not discover this at CI
   time.
2. Two-tenant RLS isolation e2e (THE test — FFS-001 §9: "if that test does not
   exist, the feature is not done"): provision a tenant-scoped table via
   /plan+/apply, insert as tenant A, assert tenant B sees nothing — adapt the
   proven harness in crates/fdb-gateway/tests/rest_rls_isolation.rs.
3. Key-rotation revocation e2e: re-run generate-keys.mjs against a scratch
   JWKS source, restart/refresh the JWKS provider, assert the OLD service_role
   key now 401s (p16-c005's refetch-on-unknown-kid makes this real).
4. OpenAPI hot-swap assertion: provisioned table appears in /openapi.json
   WITHOUT restart — valid on v1's restartRequired:true contract because
   openapi_handler loads state_manager.current() per request
   (crates/fdb-gateway/src/handlers.rs:36–38); REST routes are the only
   restart-bound surface.
5. THE phase-boundary run: full suite; drive every failure to green; record
   run count + dates in progress.json (Base Rule #18). Subsequent runs only
   for driving known failures green.

## Constraints
- This is the only change allowed to run the suite repeatedly (fix cycles).
- cargo clippy --workspace -- -D warnings must also be green here.
