# Verification — p17-c003

## Gate (FFS-001 §8 Phase 2)
Integration test against real Postgres proves the table exists on a FRESH
connection after apply; a failed statement leaves no partial table and a
failed ledger row.

## Evidence to record on completion
- [ ] `cargo check -p fdb-ports -p fdb-postgres` clean
- [ ] D7 fresh-connection test present and named in this file
- [ ] Rollback + failed-ledger test present
- [ ] Privilege-containment test present (outside-allowlist CREATE refused by
      Postgres, not just by app validation)
- [ ] grep confirms no statement text in any error/log path

## Adversarial-review dispositions (rounds 1–2, 2026-07-30)
- Ledger transition constrained to `status='planned' AND plan_hash` match;
  rows!=1 rolls the DDL back (audit invariant test added).
- persist_planned refuses silent plan_id conflicts (rows==0 -> error).
- Lost failed-transitions now surfaced in the returned apply error.
- version_after: record_version_after port method (gateway stamps it after
  the reflection refresh); apply span carries the constant role field.

## Adversarial-review round 3 (FINAL — accepted with dispositions, 2026-07-30)
See phases/p17-schema-provisioning/review/<change>/findings-r3.json. Fixed this
round: empty-IndexSpec refusal (c002); mark_failed hash-constrained +
record_version_after rows==1 check (c003). Refuted with evidence: runbook.md
length (500-line BLOCK governs source modules; runbook was 1,300+ lines across
p9–p16 §§ precedent), fdb-app sha2 (pure computation dep, not an adapter —
change-spec wording corrected). Accepted deferral: route-level auth proof
lands in c004 t7 with the same keys (rls_from_bearer is the exact function the
route gate consumes). Review loop closed at 3 rounds per max-rounds contract.
