# Verification — p17-c005

## Gate (FFS-001 §8 Phase 4)
Round-trip holds for every ColumnType, nullable and not, with and without
defaults.

## Evidence to record on completion
- [ ] `cargo check -p fdb-app -p fdb-gateway` clean
- [ ] Round-trip test enumerates all 9 ColumnType variants
- [ ] Path identifiers validated before touching the database (test with an
      injection-shaped table name -> 4xx, no query executed)
