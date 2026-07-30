# Verification — p17-c001

## Gate (FFS-001 §8 Phase 0)
The existing service_role key authenticates against a protected route and
resolves to role="service_role"; the anon key resolves to role="anon"; the
migration applies cleanly and is idempotent on re-run.

## Evidence to record on completion
- [ ] `cargo check -p fdb-gateway` clean after test addition
- [ ] 0015 applied twice against scratch DB — second run no-op, no errors
- [ ] Auth-proof test compiles; run deferred to phase boundary (c006), run
      record to be added there
- [ ] .env.example + runbook + ANON-SERVICE-ROLE-KEYS.md diffs reviewed for
      accuracy against forge-identity source (no aspirational claims)

## Adversarial-review dispositions (round 1, 2026-07-30)
- Env gating: FIXED — issuer/audience absent now skips (prerequisite), wrong
  value still fails loudly.
- "Protected route" scope: DOCUMENTED DEVIATION — c001 proves the key through
  fdb_auth::rls_from_bearer (the exact function the RLS layer and
  require_provisioner consume); full route-level 401/403 matrix is c004 t7,
  same keys. The Phase-0 gate intent (key → role claim) is covered.
- 0015 "verbatim §5": REFUTED — the added flint_provisioner ledger policy is
  a deliberate, in-file-documented fix for a spec defect (FORCE RLS with zero
  policies dead-letters every ledger write). Recorded in plan.md D-P1 area
  and the migration header.

## Adversarial-review round 3 (FINAL — accepted with dispositions, 2026-07-30)
See phases/p17-schema-provisioning/review/<change>/findings-r3.json. Fixed this
round: empty-IndexSpec refusal (c002); mark_failed hash-constrained +
record_version_after rows==1 check (c003). Refuted with evidence: runbook.md
length (500-line BLOCK governs source modules; runbook was 1,300+ lines across
p9–p16 §§ precedent), fdb-app sha2 (pure computation dep, not an adapter —
change-spec wording corrected). Accepted deferral: route-level auth proof
lands in c004 t7 with the same keys (rls_from_bearer is the exact function the
route gate consumes). Review loop closed at 3 rounds per max-rounds contract.
