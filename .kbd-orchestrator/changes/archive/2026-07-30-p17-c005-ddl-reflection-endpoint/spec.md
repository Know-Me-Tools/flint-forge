# p17-c005 — GET /schema/v1/tables/{schema}/{table}/ddl (CREATE TABLE synthesis)

**Phase:** p17-schema-provisioning
**Priority:** Round 4 — depends on c004
**Scope:** `crates/fdb-gateway/src/routes/schema/ddl.rs`, synthesis logic in fdb-app
**Source:** FFS-001 §4.4, §8 Phase 4; plan.md D-P3

## Problem

Forge cannot emit a CREATE TABLE string for an existing table, blocking
client-side registerEntityFromSql. flint_meta.columns(p_schema, p_table)
(defined via extension_sql! in crates/ext-flint-meta/src/functions.rs:107,
granted to authenticated+anon+service_role) has everything needed but no HTTP
exposure — today only fdb-reflection/src/engine.rs:127 consumes it.

## What to build

- Synthesis: query flint_meta.columns($1,$2) through the provisioner/schema
  port → render `CREATE TABLE <table> (col type [NOT NULL] [DEFAULT …], …)`
  with pk annotation; include rlsEnabled, rlsForced, schemaVersion in the
  response per §4.4. 404 for unknown table; identifiers validated before
  querying.
- Route in routes/schema/ddl.rs behind require_provisioner (D-P3: service_role
  only — the SQL-level anon grant makes the HTTP gate the boundary that
  matters) + the same 503 disabled gate.
- Round-trip property test WRITTEN (run at boundary): generate(spec) → apply →
  GET ddl → re-parse ≡ original spec for every ColumnType × nullable ×
  with/without default.

## Constraints
- Synthesis rendering is pure and unit-testable given column rows; keep the
  SQL fetch behind the port so fdb-app stays adapter-free.
