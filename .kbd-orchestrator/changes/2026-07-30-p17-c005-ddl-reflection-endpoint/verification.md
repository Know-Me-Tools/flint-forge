# Verification — p17-c005

## Gate (FFS-001 §8 Phase 4)
Round-trip holds for every ColumnType, nullable and not, with and without
defaults.

## Evidence to record on completion
- [ ] `cargo check -p fdb-app -p fdb-gateway` clean
- [ ] Round-trip test enumerates all 9 ColumnType variants
- [ ] Path identifiers validated before touching the database (test with an
      injection-shaped table name -> 4xx, no query executed)

## Implementation notes (2026-07-30)
- Data source deviation, recorded: FFS-001 §4.4 names flint_meta.columns();
  the provisioner role deliberately has no flint_meta grant (D3), so the
  adapter reads the same facts from world-readable pg_catalog (which is what
  flint_meta caches). Documented in provisioner.rs.
- Round-trip realized as: provision all 9 ColumnTypes through the real API,
  GET …/ddl, assert every column re-renders with canonical type spelling,
  nullability, and default (ddl_round_trip_covers_every_column_type).
