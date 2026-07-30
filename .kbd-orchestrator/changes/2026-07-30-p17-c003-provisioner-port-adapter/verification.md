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
