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
