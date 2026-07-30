# Verification — p17-c002

## Gate (FFS-001 §8 Phase 1)
cargo test -p fdb-app green at the phase boundary, including DDL snapshots and
negative tests for every injection shape; clippy pedantic clean.

## Evidence to record on completion
- [ ] `cargo check -p fdb-domain -p fdb-app` clean
- [ ] Injection corpus enumerated in test names (each shape its own test)
- [ ] Snapshot files reviewed: RLS block verbatim vs FFS-001 §5 template
- [ ] Hash stability test covers field reordering AND table/column order
- [ ] No public item undocumented (missing_docs deny compiles)

## Adversarial-review dispositions (round 1, 2026-07-30)
- Type/nullability drift: FIXED — PlanError::ColumnDrift refuses (both
  directions) instead of false-noop; canonical Postgres type spellings
  (e.g. `timestamp with time zone`) still satisfy the spec.
- Tenant-index name collision: FIXED — SpecError::ReservedIndexName refuses
  caller indexes named `{table}_tenant_idx` on scoped tables.
- Existing-caller-index no-op: ACCEPTED LIMITATION — TableMeta carries no
  index introspection, so indexes on existing tables re-emit with
  IF NOT EXISTS guards (replay-safe); exact replay of an applied plan is
  caught by the ledger (alreadyApplied). Documented in ddl.rs module docs;
  the earlier spec sentence claiming index no-op is superseded by this note.
