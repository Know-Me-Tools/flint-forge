# Goals

- FFS-001: expose a schema-provisioning API so apps (Sansaba Mini Apps) can declare their own data over HTTP
- POST /schema/v1/plan — pure planner: typed JSON spec in (closed ColumnType enum, no raw DDL ever), generated DDL + diff + sha256 planHash out; nothing written
- POST /schema/v1/apply — idempotent apply by planHash with drift guard (409 on recomputed-hash mismatch), one explicit BEGIN…COMMIT transaction (D7), 200+alreadyApplied on replay
- Tenant scoping generated, never caller-authored: tenant_id column, ENABLE+FORCE ROW LEVEL SECURITY, four tenant policies, tenant index, grants to authenticated (closes migrations/0013 gap)
- Caller/database authority split (D3): service_role JWT authenticates the caller; DDL executes as dedicated flint_provisioner Postgres role (NOLOGIN, no BYPASSRLS, CREATE only on allowlisted namespaces)
- Namespace allowlist FLINT_PROVISION_NAMESPACES, default-off => 503; flint_*, public, pg_*, information_schema refused unconditionally (D4)
- Audit ledger migrations/0015: flint_schema.provision_ledger records planned/applied/failed with sub + SQLSTATE only (Base Rule #18)
- GET /schema/v1/tables/{schema}/{table}/ddl — synthesize CREATE TABLE from flint_meta.columns() so clients can drive registerEntityFromSql
- GET /schema/v1/status — enabled flag, namespaces, schemaVersion, lastApply
- Auth via existing RS256 keys from sansaba-workspace generate-keys.mjs (jwks.json, iss/aud=flint-forge); require_provisioner gate: 401 no/bad token, 403 non-service_role, 503 disabled
- Additive-only v1 (D6): CREATE TABLE, ADD COLUMN (nullable/defaulted), CREATE INDEX, policies+grants; no DROP/RENAME/type-narrowing
- apply response discloses restartRequired:true (D8 route-mount gap); optional Phase 5 catch-all delegate flips it to false if benchmark holds
- Tests that gate completion: injection corpus on the generator, D7 fresh-connection commit test, auth matrix with real keys (anon=403), two-tenant RLS isolation test, key-rotation revocation test

## Source specification

Full functional spec and 6-phase implementation plan: [FFS-001-spec.md](./FFS-001-spec.md) (FFS-001, 2026-07-30). /kbd-assess and /kbd-plan should treat FFS-001 §8 as the seed change breakdown.
