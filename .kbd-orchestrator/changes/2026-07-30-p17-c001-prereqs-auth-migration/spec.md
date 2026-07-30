# p17-c001 — Phase-0 prerequisites: key wiring proof, migration 0015, env + docs + runbook

**Phase:** p17-schema-provisioning
**Priority:** Round 1 (parallel with c002) — unblocks c003+
**Scope:** `migrations/0015_flint_schema_provisioning.sql`, `.env.example`,
`docs/runbook.md`, `docs/ANON-SERVICE-ROLE-KEYS.md`, `crates/fdb-gateway/tests/`
**Source:** FFS-001 §5, §8 Phase 0, §10 — full spec at
`.kbd-orchestrator/phases/p17-schema-provisioning/FFS-001-spec.md`

## Problem

No provisioning substrate exists: no ledger, no `flint_provisioner` role, no
operator env contract, and the existing key docs misstate how auth works
(`FLINT_SERVICE_ROLE_KEY` is read by no Forge code; `forge token mint` signs
HS256 which `verify_and_build` cannot accept).

## What to build

1. `migrations/0015_flint_schema_provisioning.sql` — verbatim from FFS-001 §5:
   `flint_schema` schema, `provision_ledger` table (RLS + FORCE), partial
   unique index on `(plan_hash) WHERE status='applied'`, `flint_provisioner`
   NOLOGIN role via idempotent DO block, USAGE + SELECT/INSERT/UPDATE grants.
   NOT self-enabling. Depends on 0013/0014 conventions.
2. `.env.example`: `FLINT_PROVISION_NAMESPACES` (documented default-off) and
   `PROVISIONER_DATABASE_URL` (must connect as a role that can SET ROLE
   flint_provisioner or be it).
3. Runbook section "Schema provisioning": enable (schema + GRANT CREATE per
   namespace), disable (unset → 503), rotate (re-run generate-keys.mjs).
4. `docs/ANON-SERVICE-ROLE-KEYS.md` corrections: state plainly that no Forge
   code reads `FLINT_SERVICE_ROLE_KEY` as config, and that the working
   credential is the RS256 key minted by
   `sansaba-workspace/infra/scripts/generate-keys.mjs` (jwks.json served at
   `FLINT_GATE_JWKS_URL`, iss/aud `flint-forge`).
5. DATABASE_URL-gated auth-proof test in `crates/fdb-gateway/tests/`:
   the real Sansaba `service_role` key authenticates a protected route and
   lands `role = "service_role"` in `RlsContext`; the `anon` key lands
   `role = "anon"`; assert `FLINT_GATE_ISSUER`/`FLINT_GATE_AUDIENCE` resolve
   to `flint-forge` matching the minted claims. Test skips (does not fail)
   when keys/env are absent, matching existing gated-test convention.

## Constraints
- Integration-First (AGENTS.md): tests are WRITTEN here, the suite RUNS at the
  phase boundary (c006). `cargo check -p fdb-gateway` is the per-change verify.
- Migration must be idempotent (applies cleanly twice).
