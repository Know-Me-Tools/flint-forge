# p17-c002 — Domain spec types + pure DDL generator (the load-bearing security artifact)

**Phase:** p17-schema-provisioning
**Priority:** Round 1 (parallel with c001)
**Scope:** `crates/fdb-domain/src/provision/` (new), `crates/fdb-app/src/provision/` (new)
**Source:** FFS-001 D1/D5/D6, §4.2, §5 policy template, §8 Phase 1

## Problem

Nothing exists. The API's entire security posture rests on a closed grammar:
typed JSON spec in, generated SQL out, no caller-supplied SQL ever (D1).

## What to build

**`fdb-domain/src/provision/`** (directory module from the start — lib.rs is
326 lines; in-place addition risks the 500-line BLOCK):
- `SchemaSpec { namespace: Namespace, tables: Vec<TableSpec> }`
- `TableSpec { name, comment, tenant_scoped, acknowledge_unscoped, columns, indexes, api_exposed }`
- `ColumnSpec { name, r#type: ColumnType, nullable, primary_key, default }`
- `#[non_exhaustive] enum ColumnType` — closed 9: text|integer|bigint|numeric|boolean|date|timestamptz|uuid|jsonb
- `IndexSpec { name, columns: Vec<String>, unique }`
- `#[repr(transparent)]` newtypes: `PlanId`, `PlanHash`, `Namespace`
- Validation: `forge_domain::is_safe_identifier` on EVERY identifier
  (namespace, table, column, index, index columns); reserved namespaces
  (flint_*, public, pg_*, information_schema) refused before any allowlist;
  `tenant_id` not caller-declarable when tenant_scoped; default expression
  allowlist = literals + now() + gen_random_uuid();
  tenant_scoped:false requires acknowledge_unscoped:true (emits warning).

**`fdb-app/src/provision/ddl.rs`** — pure
`generate(spec: &SchemaSpec, live: &[TableMeta]) -> Result<Plan, PlanError>`:
- Emission (additive-only D6): CREATE SCHEMA IF NOT EXISTS; CREATE TABLE;
  ADD COLUMN (nullable or defaulted only); CREATE INDEX from caller IndexSpec
  (columns must exist in the table spec) + the generated tenant index;
  tenant RLS verbatim from FFS-001 §5 (four policies, ENABLE+FORCE, grant to
  authenticated); comments via COMMENT ON.
- Diff vs `live` → operations list with `exists` flags; `noop` when satisfied.
  A caller index that already exists diffs to no-op.
- Canonical serialization (stable field order) + sha256 → `PlanHash`.

## Constraints
- Zero infra deps in fdb-domain (serde only); fdb-app depends on domain+ports only.
- `missing_docs` is deny — every public item documented with # Errors where fallible.
- Tests WRITTEN here (snapshots incl. unique/multi-column indexes; hash
  stability across field reordering; noop; injection corpus: "; DROP TABLE",
  quoted identifiers, unicode homoglyphs, -- comments, nested $$ — including
  in index names/columns). Suite RUNS at phase boundary (c006).
