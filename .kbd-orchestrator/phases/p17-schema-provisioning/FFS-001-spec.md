# FFS-001 — Flint Forge Schema Provisioning API

**Functional specification and implementation plan**
**Target repo:** `flint-forge` · **Suggested change id:** `p17-c001-schema-provisioning`
**Date:** 2026-07-30 · **Status:** proposal, accepted as phase `p17-schema-provisioning`
**Driver:** Sansaba Workspace Mini Apps (`/Users/gqadonis/Projects/sansaba/San Saba Automation/sansaba-workspace`) need to declare and use their own data.
**Credential source:** the `service_role` / `anon` RS256 keys are minted today by `sansaba-workspace/infra/scripts/generate-keys.mjs` — no flint-gate dependency.

---

## 1. Problem

Forge is a *reflect-what-exists* gateway. It has no API by which a client can declare data it needs, and every path to creating a table runs outside the product.

Verified against the codebase, not inferred:

| Claim | Evidence |
|---|---|
| No schema/DDL/migration endpoint exists | Full mounted route set at `crates/fdb-gateway/src/bootstrap.rs:313–478`; grep across `crates/` for `/schema`, `/migrate`, `/ddl`, `/tables`, `/entity-types`, `create_table`, `execute_ddl` → zero handlers |
| Entity types come from hand-written SQL applied out of band | `sqlx::migrate!("../../migrations")` embeds files at **compile** time (`bootstrap.rs:60–67`) |
| DDL is *detected*, never *authored*, by Forge | `flint_meta.refresh_cache()` event trigger → `pg_notify('meta_runtime')` → `StateManager::run_listener` (`state_manager.rs:159–195`) |
| Forge cannot emit a `CREATE TABLE` string | `GET /openapi.json` returns lossy JSON Schema. `flint_meta.columns()` has `pg_type`, `is_nullable`, `column_default` — and is **not exposed over HTTP** |
| No tenant-scoping template for operator tables | `migrations/0013_force_rls.sql`: *"Tenant/operator-created tables are outside flint-forge's migration ownership; operators MUST apply FORCE ROW LEVEL SECURITY to their own RLS-governed tables"* |

The consequence downstream: a Sansaba Mini App that needs a new entity cannot ship without a hand-written migration in a different repo, applied by a different person, with tenant RLS re-authored by hand each time — which is exactly where isolation bugs come from.

---

## 2. Goals and non-goals

**Goals**

1. A client holding a `service_role` JWT can **declare** a table and have Forge generate correct, tenant-scoped DDL.
2. Declaration is separated from execution: a reviewable **plan**, then an idempotent **apply**.
3. Tenant `tenant_id` column, `FORCE ROW LEVEL SECURITY`, and the four RLS policies are **generated**, never authored by the caller — closing the gap `0013` disclaims.
4. Every provisioning action is recorded in an auditable ledger (Base Rule #18).
5. Forge can emit a `CREATE TABLE` string for an existing table, so a client can drive `registerEntityFromSql` at runtime.

**Non-goals**

- Arbitrary SQL execution. **No endpoint accepts a DDL string, ever.**
- Destructive operations. No `DROP TABLE`, no `DROP COLUMN`, no type narrowing in v1 — those keep going through the reviewed migration path.
- Replacing `migrations/`. Forge's own internal schema stays migration-owned.
- Cross-tenant or cross-namespace provisioning from one call.
- A general ORM or schema-diff engine.

---

## 3. Design decisions

### D1 — Structured spec only; no raw DDL, at any privilege level

The API accepts a typed JSON spec and generates SQL. It never accepts SQL.

This is the load-bearing security decision. A filtered DDL string is a parser problem you have to win every time; a closed grammar is a problem you win once. Identifiers go through the existing `forge_domain::is_safe_identifier` (already used at `fdb-postgres/src/backend.rs:88` for `SET LOCAL ROLE`), and column types come from a closed Rust enum rather than free text — so an injection attempt fails type-checking at deserialization, before any code runs.

**Rejected:** an `"escapeHatch": "raw_sql"` field for advanced cases. Once it exists, every caller uses it and the guarantee is gone. Advanced cases go through `migrations/`.

### D2 — Plan / apply, not a single mutating call

`POST /schema/v1/plan` is a pure function: spec in, generated DDL and a diff out, nothing written. `POST /schema/v1/apply` takes a plan hash and executes it.

Reasoning: DDL is close to irreversible (Base Rule #8). A plan is reviewable by a human or by CI, diffable in a PR, and the hash makes apply idempotent — replaying it is a no-op rather than a second `ALTER`. It also lets the Sansaba side generate a plan at build time and apply it at deploy time, which is where provisioning belongs.

### D3 — Caller authority and database authority are different things

The caller proves `service_role` **via JWT** — the existing RS256 key from `generate-keys.mjs` (§4.1), carrying `role: "service_role"`. The DDL executes as a dedicated `flint_provisioner` Postgres role.

They must be separate:

- `service_role` has `BYPASSRLS` (`migrations/0014`) but is **not** a table owner and likely lacks `CREATE` on the target schema.
- `state_manager.rs:38–43` explicitly forbids reusing the migration-owner pool: *"MUST be backed by a non-owner Postgres role distinct from `engine`'s privileged introspection/catalog pool; never the migration-owner pool."*
- The blast radius of a compromised JWT should be "can create tables in allowlisted namespaces", not "owns the database".

`flint_provisioner` gets `CREATE` on allowlisted namespaces only, and nothing else.

This split carries more weight than it first appears. The `service_role` credential is a **10-year key**, so token lifetime contributes nothing to containment — the Postgres role boundary and the namespace allowlist are the *only* things standing between a leaked key and the database. Design them as if the key is already public.

### D4 — Namespace allowlist, operator-controlled

`FLINT_PROVISION_NAMESPACES` (comma-separated) is the set of schemas provisioning may touch. Empty or unset ⇒ the feature is **off** and every endpoint returns `503`. `flint_*`, `public`, `pg_*`, and `information_schema` are permanently refused regardless of configuration.

Default-off matters: this ships into existing deployments, and an operator who has not opted in should not gain a DDL surface by upgrading.

### D5 — Tenant scoping is generated, not requested

`"tenantScoped": true` produces, verbatim and unconditionally:

```sql
tenant_id text NOT NULL
ALTER TABLE … ENABLE ROW LEVEL SECURITY;
ALTER TABLE … FORCE ROW LEVEL SECURITY;
-- four policies: select / insert / update / delete
CREATE INDEX … ON … (tenant_id);
GRANT SELECT, INSERT, UPDATE, DELETE ON … TO authenticated;
```

The caller cannot supply, override, or disable a policy body. The most common multi-tenant bug is a table that ships without RLS because someone forgot; making it unforgettable is most of the value of this whole change.

### D6 — Additive operations only in v1

| v1 | Deferred |
|---|---|
| `CREATE TABLE` (new) | `DROP TABLE` |
| `ADD COLUMN` (nullable, or with default) | `DROP COLUMN` |
| `CREATE INDEX` | type changes / narrowing |
| policy + grant creation | `RENAME` |

Every v1 operation is safe to replay and safe to abandon halfway. Destructive changes need a review workflow this API deliberately does not have.

### D7 — Apply must commit explicitly

`PgBackend::acquire` issues `BEGIN` and leaves the transaction open for the connection's lifetime (`conn.rs:28–56`: *"without a `COMMIT`, deadpool rolls it back when the object is recycled and every write is silently discarded"*).

The provisioning adapter therefore uses its **own** pool and its own transaction discipline. It does not route DDL through `DatabaseBackend::acquire`. Getting this wrong produces the worst possible failure: a `200 OK` and no table.

### D8 — Route availability after apply is a known, disclosed gap

On apply, the `flint_meta` event trigger fires, `meta_runtime` notifies, `StateManager` recompiles, and `ArcSwap` installs the new `CompiledState`. That makes the new table visible **immediately** in `/openapi.json`, `/mcp/v1/tools`, GraphQL subscriptions, and the A2UI catalog.

It does **not** create REST routes. `bootstrap.rs:298–300`:

> *"Route hot-reload note: the reflection router is mounted once at startup. Handler bodies read from RestState (captured at compile time); DDL-driven route-set changes require a catch-all delegate pattern (future enhancement)."*

v1 tells the truth about this: the apply response carries `"restartRequired": true`. Phase 5 implements the delegate and flips it to `false`. Pretending otherwise would mean a client gets `200 OK` and then `404` on the table it just created.

---

## 4. API specification

Base path `/schema/v1`. Every endpoint requires a `service_role` JWT. Every response is JSON.

### 4.1 Authentication

```
Authorization: Bearer <RS256/ES256 JWT with "role": "service_role">
```

Verified by `forge_identity::verify_and_build` against `FLINT_GATE_JWKS_URL` / `FLINT_GATE_ISSUER` / `FLINT_GATE_AUDIENCE`, then role-checked by a new `require_provisioner` helper modelled on `fke-server/src/handlers/admin.rs:19–49`.

| Condition | Status |
|---|---|
| No `Authorization` header | `401 missing Authorization header` |
| Signature/issuer/audience/expiry failure | `401 invalid or expired token` — note the shipped key is 10-year, so in practice this fires on a **rotation** (`kid` no longer in the JWKS), not on expiry |
| Valid token, `role != "service_role"` | `403 provisioner role required` |
| `FLINT_PROVISION_NAMESPACES` unset/empty | `503 schema provisioning is not enabled` |

> **The `service_role` key already exists — no new minting work is required.**
>
> `sansaba-workspace/infra/scripts/generate-keys.mjs` implements
> `FLINT_ANON_SERVICE_ROLE_KEYS_SPEC.md` §3.1 (`forge keygen init`) and produces
> exactly what this API needs:
>
> | Property | Value | Matches Forge? |
> |---|---|---|
> | Algorithm | RS256 with a `kid` header | ✅ `verify_and_build` accepts RS256 and requires `kid` |
> | Claims | `role: "service_role"`, `principal_type: "Service"`, `sub`, `iss`, `aud`, `exp`, `jti` | ✅ `forge_identity::Claims.role` is exactly what `SET LOCAL ROLE` consumes |
> | Public half | `infra/keys/jwks.json` | ✅ fetched via `FLINT_GATE_JWKS_URL` |
> | `iss` / `aud` | both `flint-forge` | ✅ matches `FLINT_GATE_ISSUER` / `FLINT_GATE_AUDIENCE` in `server/.env.example` |
> | Expiry | 10 years | API-key shaped, not request-scoped |
>
> It also emits an `anon` counterpart (`role: "anon"`, RLS-gated) and writes both
> to a git-ignored `.env.keys`. The keys are already generated in that checkout.
>
> The script's own header explains the one deviation from the spec — the spec
> defaults to HS256, and RS256 was chosen because *"a shared secret has no public
> half to publish, so an HS256 key can never satisfy forge's verification path."*
> That reasoning is correct and is why the HS256 path is not an option here either.
>
> **Two caveats that are still true.** First, this utility lives in
> `sansaba-workspace`, not in flint-gate or flint-forge — it is project-local, so a
> second consumer would need it promoted to a platform tool (see §11.6). Second,
> **flint-gate itself genuinely cannot do this**: it has no CLI subcommand at all,
> its default `signing_algorithm` is HS256 (so its JWKS serves `{"keys":[]}`), its
> mint path never sets a `role` claim — `client_credentials` sets `flint_kind:
> "service"` instead — and its default TTL is 300s. In flint-gate, `role` travels
> in the trusted `X-Flint-Role` header, not in the token. That is a different
> mechanism from what Forge reads, and reconciling the two is out of scope here.

### 4.2 `POST /schema/v1/plan`

Pure. Computes generated DDL and a diff against the live schema. Writes nothing.

**Request**

```jsonc
{
  "namespace": "sansaba_sourcing",
  "tables": [
    {
      "name": "permit_watch",
      "comment": "Texas RRC permits a user is watching",
      "tenantScoped": true,
      "columns": [
        { "name": "id",         "type": "text",        "nullable": false, "primaryKey": true },
        { "name": "api_number", "type": "text",        "nullable": false },
        { "name": "operator",   "type": "text",        "nullable": true  },
        { "name": "filed_at",   "type": "timestamptz", "nullable": true  },
        { "name": "payload",    "type": "jsonb",       "nullable": false, "default": "'{}'" },
        { "name": "created_at", "type": "timestamptz", "nullable": false, "default": "now()" }
      ],
      "indexes": [
        { "name": "permit_watch_api_idx", "columns": ["api_number"], "unique": false }
      ],
      "apiExposed": true
    }
  ]
}
```

`type` is a closed enum: `text | integer | bigint | numeric | boolean | date | timestamptz | uuid | jsonb`. `default` accepts only literals and the allowlisted functions `now()`, `gen_random_uuid()`.

**Response `200`**

```jsonc
{
  "planId": "pln_01JQ8…",
  "planHash": "sha256:9f2c…",
  "namespace": "sansaba_sourcing",
  "operations": [
    { "kind": "create_schema", "target": "sansaba_sourcing", "exists": false },
    { "kind": "create_table",  "target": "sansaba_sourcing.permit_watch", "exists": false },
    { "kind": "enable_rls",    "target": "sansaba_sourcing.permit_watch" },
    { "kind": "create_policy", "target": "permit_watch_tenant_select" },
    { "kind": "create_policy", "target": "permit_watch_tenant_insert" },
    { "kind": "create_policy", "target": "permit_watch_tenant_update" },
    { "kind": "create_policy", "target": "permit_watch_tenant_delete" },
    { "kind": "create_index",  "target": "permit_watch_tenant_idx" },
    { "kind": "create_index",  "target": "permit_watch_api_idx" },
    { "kind": "grant",         "target": "authenticated" }
  ],
  "ddl": "-- generated by FFS-001 …\nCREATE SCHEMA IF NOT EXISTS …",
  "warnings": [],
  "noop": false,
  "expiresAt": "2026-07-31T10:00:00Z"
}
```

`noop: true` when the live schema already satisfies the spec. Plans expire after 24h so a stale plan cannot be applied against a drifted schema.

### 4.3 `POST /schema/v1/apply`

```jsonc
{ "planHash": "sha256:9f2c…" }
```

Re-plans from the stored spec and **refuses if the recomputed hash differs** — that is the drift guard. Executes in one transaction, commits, records the ledger row.

**Response `200`**

```jsonc
{
  "planId": "pln_01JQ8…",
  "applied": true,
  "alreadyApplied": false,
  "schemaVersionBefore": 41,
  "schemaVersionAfter": 42,
  "reflectionRefreshed": true,
  "restartRequired": true,
  "restartNote": "REST routes for new tables are mounted at startup; OpenAPI, MCP tools, GraphQL subscriptions and the A2UI catalog are live now."
}
```

| Condition | Status |
|---|---|
| Unknown or expired `planHash` | `404` / `410` |
| Schema drifted since plan | `409 plan no longer matches live schema` |
| Namespace not allowlisted | `403` |
| Already applied (same hash) | `200` with `alreadyApplied: true` |
| DDL failed | `500`, transaction rolled back, ledger row `status: "failed"` with the Postgres `SQLSTATE` (never the raw statement) |

### 4.4 `GET /schema/v1/tables/{schema}/{table}/ddl`

Synthesizes a `CREATE TABLE` string from `flint_meta.columns()` — the gap that stops a client from driving `registerEntityFromSql` at runtime.

```jsonc
{
  "schema": "sansaba_sourcing",
  "table": "permit_watch",
  "ddl": "CREATE TABLE permit_watch (\n  id text NOT NULL,\n  …\n);",
  "rlsEnabled": true,
  "rlsForced": true,
  "schemaVersion": 42
}
```

Read-only. Could arguably be `authenticated` rather than `service_role`, since `flint_meta.columns()` is already granted to `authenticated` — but v1 keeps it behind the same gate, because a full column list is a better reconnaissance target than a JSON Schema. Revisit with evidence.

### 4.5 `GET /schema/v1/status`

```jsonc
{
  "enabled": true,
  "namespaces": ["sansaba_sourcing", "sansaba_reports"],
  "schemaVersion": 42,
  "lastApply": { "planId": "pln_01JQ8…", "at": "2026-07-30T09:12:00Z", "status": "applied" }
}
```

---

## 5. Data model

New migration `migrations/0015_flint_schema_provisioning.sql` (next free number — there is no `0001`).

```sql
-- Migration: 0015_flint_schema_provisioning.sql
-- FFS-001 — ledger and dedicated role for the schema-provisioning API.
--
-- Two things land here and nothing else: an append-mostly audit ledger, and a
-- non-owner role that may CREATE only inside operator-allowlisted namespaces.
-- The role deliberately does NOT get BYPASSRLS and is NOT a table owner: the
-- caller's authority (a service_role JWT) and the database's authority
-- (flint_provisioner) are separate blast radii by design.
--
-- Depends on: 0013_force_rls (FORCE RLS convention), 0014_service_role_bypassrls.
-- Idempotent: IF NOT EXISTS guards throughout; the DO block is a no-op when the
-- role already exists.
--
-- NOT self-enabling: creating the role grants nothing. An operator must also set
-- FLINT_PROVISION_NAMESPACES and GRANT CREATE per namespace (see docs/runbook).
CREATE SCHEMA IF NOT EXISTS flint_schema;
CREATE TABLE IF NOT EXISTS flint_schema.provision_ledger (
    plan_id        text        PRIMARY KEY,
    plan_hash      text        NOT NULL,
    namespace      text        NOT NULL,
    spec           jsonb       NOT NULL,
    generated_ddl  text        NOT NULL,
    status         text        NOT NULL CHECK (status IN ('planned','applied','failed')),
    applied_by     text,                      -- JWT `sub`. Never the raw bearer.
    applied_at     timestamptz,
    version_before bigint,
    version_after  bigint,
    error_code     text,                      -- SQLSTATE only, never the statement
    created_at     timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX IF NOT EXISTS provision_ledger_hash_applied_idx
    ON flint_schema.provision_ledger (plan_hash)
    WHERE status = 'applied';                 -- makes apply idempotent at the DB
ALTER TABLE flint_schema.provision_ledger ENABLE ROW LEVEL SECURITY;
ALTER TABLE flint_schema.provision_ledger FORCE ROW LEVEL SECURITY;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'flint_provisioner') THEN
        CREATE ROLE flint_provisioner NOLOGIN;
    END IF;
END
$$;
GRANT USAGE ON SCHEMA flint_schema TO flint_provisioner;
GRANT SELECT, INSERT, UPDATE ON flint_schema.provision_ledger TO flint_provisioner;
```

**The generated tenant policy template** — this is the artifact that closes the `0013` gap:

```sql
CREATE POLICY {t}_tenant_select ON {ns}.{t} FOR SELECT TO authenticated
  USING (tenant_id = current_setting('request.jwt.claims', true)::json ->> 'tenant_id');
CREATE POLICY {t}_tenant_insert ON {ns}.{t} FOR INSERT TO authenticated
  WITH CHECK (tenant_id = current_setting('request.jwt.claims', true)::json ->> 'tenant_id');
CREATE POLICY {t}_tenant_update ON {ns}.{t} FOR UPDATE TO authenticated
  USING      (tenant_id = current_setting('request.jwt.claims', true)::json ->> 'tenant_id')
  WITH CHECK (tenant_id = current_setting('request.jwt.claims', true)::json ->> 'tenant_id');
CREATE POLICY {t}_tenant_delete ON {ns}.{t} FOR DELETE TO authenticated
  USING (tenant_id = current_setting('request.jwt.claims', true)::json ->> 'tenant_id');
```

Matches the pattern already proven in the Sansaba replica (`infra/replica/01-schema.sql:211–252`).

---

## 6. Security model

| Threat | Mitigation |
|---|---|
| SQL injection via spec | No SQL is accepted. Closed type enum + `forge_domain::is_safe_identifier` on every identifier. Injection fails at deserialization. |
| Privilege escalation to owner | DDL runs as `flint_provisioner` — `NOLOGIN`, no `BYPASSRLS`, `CREATE` only on allowlisted namespaces |
| Provisioning Forge's own tables | `flint_*`, `public`, `pg_*`, `information_schema` refused unconditionally, before the allowlist check |
| Table ships without RLS | Impossible for `tenantScoped: true` — policies are generated. `tenantScoped: false` requires explicit `"acknowledgeUnscoped": true` and emits a warning into the plan and the ledger. |
| Stolen `service_role` JWT | Scope-limited to `CREATE` in allowlisted namespaces, additive only. Every action attributed to `sub` in the ledger. **Expiry is not a control here** — the key from `generate-keys.mjs` is 10-year; containment is the `flint_provisioner` role split (D3) plus the namespace allowlist (D4), and revocation is a key rotation (re-run the script), not a token timeout. |
| `anon` key reaching a provisioning route | `require_provisioner` refuses `role != "service_role"` with `403`. Phase 0's gate tests this explicitly rather than assuming it — the `anon` key is publishable and will end up in browsers. |
| A single long-lived credential holding permanent DDL rights | Acknowledged and open (§11.7). Today the allowlist bounds it; a separate short-lived provisioning token is the stronger answer if the standing grant proves too broad. |
| Replay / double-apply | Content hash + partial unique index on `(plan_hash) WHERE status='applied'` |
| Apply against drifted schema | Recompute-and-compare hash at apply; `409` on mismatch |
| Secret leakage in logs | Ledger stores `sub` and `SQLSTATE` only. `RlsContext.raw_bearer`, `keto_subject`, `vault_key_id` are already documented `MUST NOT` log — the new tracing spans record `plan_id`, `namespace`, `role` and nothing else. |
| Silent rollback (D7) | Adapter owns its transaction and commits explicitly; an integration test asserts the table exists **on a fresh connection** after apply |

---

## 7. Architecture placement

Hexagonal rule: domain and app crates never import adapters; composition happens only in `fdb-gateway`.

| Crate | Adds | Why there |
|---|---|---|
| `fdb-domain` | `SchemaSpec`, `TableSpec`, `ColumnSpec`, `ColumnType`, `IndexSpec`, `PlanId`, `PlanHash`, `Namespace` | Layer 0, serde only, zero infra. `#[repr(transparent)]` newtypes; `#[non_exhaustive]` on the type enum |
| `fdb-ports` | `SchemaProvisioner` trait | The only DDL-adjacent seam. Sits beside `SchemaProvider`, which introspects; this one mutates. |
| `fdb-app` | `provision::{plan, apply}` + **the DDL generator** | The generator is a pure function `SchemaSpec → Ddl`. Putting it here makes the highest-risk code unit-testable with no database. |
| `fdb-postgres` | `PgProvisioner` + its own pool | Adapter. Owns the transaction discipline D7 requires. |
| `fdb-gateway` | `routes/schema/`, `require_provisioner` | Composition root; the only crate that knows both. |

Port sketch, matching the existing `fdb-ports` style (`async_trait`, `thiserror`, `#[non_exhaustive]`):

```rust
/// Applies generated, validated DDL. Never accepts caller-supplied SQL.
#[async_trait]
pub trait SchemaProvisioner: Send + Sync {
    /// Live column/table state for the namespaces this provisioner may touch.
    async fn introspect_namespace(&self, ns: &Namespace)
        -> Result<Vec<TableMeta>, BackendError>;
    /// Execute a validated plan in one transaction and commit.
    ///
    /// # Errors
    /// Returns [`BackendError::Query`] carrying SQLSTATE only — never the
    /// rendered statement, which may embed operator identifiers.
    async fn apply(&self, plan: &ValidatedPlan)
        -> Result<AppliedPlan, BackendError>;
}
```

The generator signature that carries the test weight:

```rust
// fdb-app/src/provision/ddl.rs
pub fn generate(spec: &SchemaSpec, live: &[TableMeta]) -> Result<Plan, PlanError>;
```

Pure, deterministic, snapshot-testable.

---

## 8. Implementation plan

Six phases. Each ends at a hard gate; do not proceed past a failing gate.

### Phase 0 — Prerequisites *(no external blocker)*

| # | Task |
|---|---|
| 0.1 | Point Forge at the existing key set: `FLINT_GATE_JWKS_URL` → the served `infra/keys/jwks.json`, `FLINT_GATE_ISSUER=flint-forge`, `FLINT_GATE_AUDIENCE=flint-forge`. **No minting work — `generate-keys.mjs` already produces the token** (§4.1) |
| 0.2 | Confirm end to end that the existing `FLINT_SERVICE_ROLE_KEY` authenticates against a protected Forge route and lands `role = "service_role"` in `RlsContext` |
| 0.3 | Correct `flint-forge/docs/ANON-SERVICE-ROLE-KEYS.md`: `FLINT_SERVICE_ROLE_KEY` is read by no code *in Forge*, and `forge token mint`'s HS256 output cannot authenticate — the working key comes from `generate-keys.mjs` |
| 0.4 | Write `migrations/0015_flint_schema_provisioning.sql` (§5) |
| 0.5 | Add `FLINT_PROVISION_NAMESPACES` + `PROVISIONER_DATABASE_URL` to `.env.example` and the runbook |
| 0.6 | Operator runbook section: create the role, `GRANT CREATE ON SCHEMA … TO flint_provisioner` per namespace |

**Gate:** the existing `service_role` key authenticates against a protected route and resolves to `role = "service_role"`; the `anon` key resolves to `role = "anon"` and is refused by `require_provisioner` with `403`; migration applies cleanly and is idempotent on re-run.

### Phase 1 — Domain types and the DDL generator *(no database)*

| # | Task |
|---|---|
| 1.1 | `fdb-domain`: the spec types, `#[non_exhaustive]` `ColumnType`, transparent newtypes |
| 1.2 | Validation: identifier safety, reserved-namespace refusal, reserved column names (`tenant_id` may not be caller-declared when `tenantScoped`), default-expression allowlist |
| 1.3 | `fdb-app/src/provision/ddl.rs` — `generate()` |
| 1.4 | Tenant policy/index/grant emission from the template |
| 1.5 | Diff against `live: &[TableMeta]` → operation list, `noop` detection |
| 1.6 | Canonical serialization + `sha256` plan hash (stable field ordering, or the hash is meaningless) |

**Gate:** `cargo test -p fdb-app` green, including snapshot tests of generated DDL and **negative tests for every injection shape** (`"; DROP TABLE"`, quoted identifiers, unicode homoglyphs, `--` comment terminators, nested `$$`). `cargo clippy --workspace -- -D warnings` clean.

### Phase 2 — Port and adapter

| # | Task |
|---|---|
| 2.1 | `fdb-ports`: `SchemaProvisioner` |
| 2.2 | `fdb-postgres`: `PgProvisioner` with its **own** pool from `PROVISIONER_DATABASE_URL` |
| 2.3 | Transaction: `BEGIN` → statements → **`COMMIT`** (D7) |
| 2.4 | Ledger writes: `planned` → `applied` \| `failed` |
| 2.5 | Error mapping: SQLSTATE only into `BackendError::Query` |
| 2.6 | `tracing` span at the port boundary: `plan_id`, `namespace`, `role` — nothing else |

**Gate:** integration test against a real Postgres proves a table exists **on a fresh connection** after apply (the D7 regression test), and that a failed statement leaves no partial table and a `failed` ledger row.

### Phase 3 — Gateway routes

| # | Task |
|---|---|
| 3.1 | `require_provisioner(&HeaderMap) -> Result<RlsContext, Response>`, copying the `fke-server` admin idiom verbatim in shape |
| 3.2 | `routes/schema/{mod,plan,apply,ddl,status}.rs` — split now; the 500-line rule bites late otherwise |
| 3.3 | Feature-gate mounting on `FLINT_PROVISION_NAMESPACES` being non-empty; `503` when off |
| 3.4 | Merge the router in `bootstrap.rs` following the MCP-group idiom (`.route` × n → `.with_state` → `.merge`) |
| 3.5 | Plan store with 24h expiry |
| 3.6 | OpenAPI documentation for the new routes |

**Gate:** route tests cover 401 (no header), 401 (bad signature), 403 (`authenticated` role), 503 (disabled), 200 (plan), 200 (apply), 200 + `alreadyApplied` (replay), 409 (drift), 403 (reserved namespace).

### Phase 4 — DDL reflection endpoint

| # | Task |
|---|---|
| 4.1 | Synthesize `CREATE TABLE` from `flint_meta.columns()` |
| 4.2 | Round-trip property test: `generate(spec)` → apply → `GET …/ddl` → re-parse ≡ original spec |

**Gate:** round-trip holds for every `ColumnType`, nullable and not, with and without defaults.

### Phase 5 — Route catch-all delegate *(optional; flips `restartRequired`)*

| # | Task |
|---|---|
| 5.1 | Replace the one-time `.merge(reflection_router)` with a fallback handler that loads `state_manager.current().router` per unmatched request and delegates via `tower::Service` |
| 5.2 | Preserve the `require_rls` layer on the delegated path |
| 5.3 | Benchmark the added `ArcSwap::load` + router clone on the unmatched path |
| 5.4 | Flip `restartRequired` to `false` |

**Gate:** a table created via `/schema/v1/apply` is reachable at `/{ns}/{table}` **without restart**, with no measurable regression on already-mounted routes. If the benchmark regresses, ship without Phase 5 and keep `restartRequired: true` — an honest flag beats a slow gateway.

---

## 9. Testing strategy

| Level | Coverage |
|---|---|
| Unit (`fdb-domain`) | Identifier validation, type enum round-trip, reserved-name refusal |
| Unit (`fdb-app`) | DDL snapshots; diff/noop; hash stability across field reordering; the full injection corpus |
| Integration (`fdb-postgres`) | Real Postgres: apply commits; failure rolls back; ledger transitions; **`flint_provisioner` cannot create outside the allowlist** |
| Route (`fdb-gateway`) | The auth matrix in Phase 3's gate, driven by the **real keys** from `.env.keys` — `FLINT_SERVICE_ROLE_KEY` passes, `FLINT_ANON_KEY` gets `403`. Use the actual credentials rather than synthetic tokens; a hand-rolled test token can accidentally satisfy a check the shipped key would fail. |
| End-to-end | Provision a table → assert it appears in `/openapi.json` after hot-swap → assert RLS actually isolates two tenants |

The tenant-isolation test is the one that matters most: two JWTs, two `tenant_id`s, one table, and tenant B must not see tenant A's row. If that test does not exist, the feature is not done.

A second test worth writing early: **rotate and confirm the old token dies.** Re-run `generate-keys.mjs`, restart the JWKS source, and assert a request bearing the previous `service_role` key now fails with `401`. Since expiry is not a control on a 10-year key, rotation is the revocation path — and an untested revocation path is not one you can rely on during an incident.

---

## 10. Operator instructions

**Enable (once per deployment):**

```bash
# 1. Apply migrations (runs automatically at gateway startup)
#    0015 creates flint_schema.provision_ledger and the flint_provisioner role.
# 2. Create each namespace and grant CREATE — the role gets nothing by default.
psql "$DATABASE_URL" <<'SQL'
CREATE SCHEMA IF NOT EXISTS sansaba_sourcing;
GRANT USAGE, CREATE ON SCHEMA sansaba_sourcing TO flint_provisioner;
GRANT USAGE ON SCHEMA sansaba_sourcing TO authenticated;
SQL
# 3. Turn the API on.
export FLINT_PROVISION_NAMESPACES="sansaba_sourcing,sansaba_reports"
export PROVISIONER_DATABASE_URL="postgres://flint_provisioner@…/forge"
# 4. Restart the gateway. Unset FLINT_PROVISION_NAMESPACES to disable — the
#    routes return 503 and no DDL path exists.
```

**Use:**

```bash
# The service_role key already exists. Generate it once if you have not:
#   node infra/scripts/generate-keys.mjs      # writes infra/keys/ + .env.keys
# Re-running ROTATES: it regenerates the keypair and invalidates prior tokens.
source .env.keys && TOKEN="$FLINT_SERVICE_ROLE_KEY"
curl -sS -X POST https://forge.example/schema/v1/plan \
  -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d @permit-watch.spec.json | tee plan.json          # review plan.json.ddl
curl -sS -X POST https://forge.example/schema/v1/apply \
  -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d "{\"planHash\":\"$(jq -r .planHash plan.json)\"}"
```

Review the `ddl` field before applying. That is the whole point of the plan step — and in CI, `plan.json` belongs in the PR.

---

## 11. Open questions

1. **Does `/schema/v1/tables/…/ddl` need `service_role`?** `flint_meta.columns()` is already granted to `authenticated`. Keeping it privileged is defensible caution, not a reasoned position — decide with a threat case, not a default.
2. **Where does the plan store live?** In-memory (simple, lost on restart, breaks plan-then-apply across a deploy) or in `provision_ledger` with `status='planned'` (durable, needs expiry sweeping). Leaning durable.
3. **Should `tenantScoped: false` be allowed at all in v1?** Every Sansaba use case is tenant-scoped. Refusing it outright removes the acknowledgement flag and one failure mode.
4. **Phase 5 cost.** The delegate adds work to every unmatched request. Needs a number before committing.
5. **Namespace-per-module or shared?** `sansaba_<module>` isolates cleanly but multiplies grants. A single `sansaba_modules` schema with a `mini_app_id` column is cheaper and weaker. This is a Sansaba decision, not a Forge one.
6. **Promote `generate-keys.mjs` to a platform utility?** It currently lives in `sansaba-workspace`, so a second Forge consumer would have to copy it — and copied key-minting is how key material ends up in the wrong repo. Candidates: a `forge keygen` subcommand in `forge-cli` (the spec's original `forge keygen init` naming), or a flint-gate CLI subcommand. The latter is the larger job: flint-gate would also need its default `signing_algorithm` moved off HS256 and a `role` claim added to the mint path, since today `role` travels in the `X-Flint-Role` header instead.
7. **Should the 10-year key be the one that holds DDL rights?** A long-lived key is right for `anon`, and defensible for `service_role` reads. Handing the *same* credential permanent `CREATE` is a larger standing grant than it looks. Worth considering a separate short-lived provisioning token, or accepting it because the namespace allowlist already bounds the damage.

---

## 12. Relationship to the Sansaba Mini-App contract

This API is the "Option A" provisioning path behind a Mini App manifest's `provides.entities[]` block. The manifest declares; a build- or deploy-time step calls `/plan`; a reviewer reads the DDL; CI calls `/apply`. **A manifest never calls this API at runtime** — declaration is not execution, and the manifest is a privilege document.

Sansaba's cheaper alternative remains valid and should ship first: a single `mini_app_record` table with RLS on `(tenant_id, mini_app_id)` covers annotation, checklist state, and saved views with no DDL, no restart, and no dependency on this specification.

**One coupling to name explicitly.** The credential this entire API depends on is minted by `sansaba-workspace/infra/scripts/generate-keys.mjs` — a script in the *consuming* repo, not in Forge or in flint-gate. That is fine while Sansaba is the only consumer and it is why Phase 0 has no external blocker. It stops being fine the moment a second product needs a `service_role` key, because the alternative to promoting the script is copying it, and copied key-minting is how private keys end up in the wrong repository. §11.6 tracks the decision; the trigger for making it is the second consumer, not a calendar date.

---

*Prometheus AGS · FFS-001 · targets `flint-forge`; the `service_role` credential is minted today by `sansaba-workspace/infra/scripts/generate-keys.mjs` — no flint-gate dependency*
