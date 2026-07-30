# Schema Provisioning API (`/schema/v1`)

> Shipped in p17 (FFS-001). Design rationale and as-built decision record:
> [`docs/FFS-001-SCHEMA-PROVISIONING.md`](../FFS-001-SCHEMA-PROVISIONING.md).
> Operator enablement: [`docs/runbook.md` §14](../runbook.md).
> Credentials: [`docs/ANON-SERVICE-ROLE-KEYS.md`](../ANON-SERVICE-ROLE-KEYS.md).

The Schema Provisioning API lets an application holding a `service_role` JWT
**declare** the tables it needs and have Forge **generate and apply** correct,
tenant-scoped DDL. It is the productized alternative to hand-written
out-of-band migrations with hand-authored RLS.

Two properties define the whole surface:

1. **No endpoint accepts SQL — ever.** You send a typed JSON spec; Forge
   generates the DDL. Injection attempts fail at deserialization/validation,
   before any code runs.
2. **Declaration is separated from execution.** `plan` produces a reviewable
   artifact (the exact DDL plus a content hash); `apply` executes only a plan
   whose hash still matches reality.

## Availability

**Default off.** All four endpoints return `503 schema provisioning is not
enabled` until an operator sets `FLINT_PROVISION_NAMESPACES` (and
`PROVISIONER_DATABASE_URL`) and restarts the gateway. The routes are always
mounted, so a disabled deployment answers `503` — never `404`.

| Env var | Meaning |
|---|---|
| `FLINT_PROVISION_NAMESPACES` | Comma-separated schema allowlist. Empty/unset = feature off. |
| `PROVISIONER_DATABASE_URL` | Connection string for the provisioner's own pool (a LOGIN role that can `SET ROLE flint_provisioner`). Startup fails closed if the allowlist is set without it. |

`flint_*`, `public`, `pg_*`, and `information_schema` are refused
unconditionally, before the allowlist is consulted.

## Authentication

```
Authorization: Bearer <RS256 JWT with "role": "service_role">
```

Verified by `forge-identity` against `FLINT_GATE_JWKS_URL` /
`FLINT_GATE_ISSUER` / `FLINT_GATE_AUDIENCE`, then role-gated.

| Condition | Response |
|---|---|
| No `Authorization` header | `401 missing Authorization header` |
| Signature / issuer / audience / expiry failure | `401 invalid or expired token` |
| Valid token, `role != "service_role"` (e.g. the `anon` key) | `403 provisioner role required` |
| Feature disabled | `503 schema provisioning is not enabled` |

The `service_role` key is 10-year: expiry is not a control. Revocation is key
rotation, and revocation latency equals the JWKS cache TTL
(`FLINT_GATE_JWKS_TTL_SECS`, default 600s) — see
[Key management](../ANON-SERVICE-ROLE-KEYS.md#rotation).

## The spec grammar

```jsonc
{
  "namespace": "acme_sourcing",          // single-segment identifier, allowlisted
  "tables": [
    {
      "name": "permit_watch",
      "comment": "Permits a user is watching",   // optional; escaped, emitted as COMMENT ON
      "tenantScoped": true,                       // see Tenant scoping below
      "columns": [
        { "name": "id",         "type": "uuid",        "nullable": false,
          "primaryKey": true,  "default": "gen_random_uuid()" },
        { "name": "api_number", "type": "text",        "nullable": false },
        { "name": "filed_at",   "type": "timestamptz", "nullable": true },
        { "name": "payload",    "type": "jsonb",       "nullable": false, "default": "'{}'" }
      ],
      "indexes": [
        { "name": "permit_watch_api_idx", "columns": ["api_number"], "unique": false }
      ]
    }
  ]
}
```

**Column types** are a closed enum — free-text types are rejected at parse
time:

`text` · `integer` · `bigint` · `numeric` · `boolean` · `date` ·
`timestamptz` · `uuid` · `jsonb`

**Defaults** accept only inert literals and two allowlisted functions:

- quoted string with no embedded quote/backslash/control chars: `'{}'`, `'draft'`, `'2024-01-01'`
- integer / decimal: `0`, `-3.5`
- `true` / `false`
- `now()` · `gen_random_uuid()`

**Validation rules** (each produces a named `400`):

- every identifier (namespace, table, column, index, index column) must pass
  the safe-identifier check: ASCII letter/underscore start, ASCII
  alphanumerics/underscores only, no reserved keywords, no dots
- `tenant_id` may not be caller-declared on a tenant-scoped table (the
  generator owns it)
- caller indexes may not use the generated `{table}_tenant_idx` name, must
  declare at least one column, and every column must exist on the table
- `tenantScoped: false` requires an explicit `"acknowledgeUnscoped": true`
  and records a warning in the plan and the ledger — and note that a table
  without RLS is **not exposed** by Forge's reflection surfaces at all

**Additive-only.** The generator can emit `CREATE TABLE`, guarded
`ADD COLUMN` (nullable or defaulted only), `CREATE INDEX`, RLS policies and
grants. No `DROP`, no `RENAME`, no type changes exist as emissions. A spec
whose column types/nullability differ from the live table is **refused**
(never silently treated as satisfied) with a pointer at the reviewed
migration path.

**Schema lifecycle is operator-owned.** Plans never create schemas — the
provisioner role deliberately cannot (see the runbook §14 enablement steps).
Planning into an allowlisted namespace whose schema does not exist yet
returns `409` with the operator instruction.

### Tenant scoping (`tenantScoped: true`)

Generated verbatim and unconditionally — the caller cannot supply, override,
or disable any part of it:

```sql
tenant_id text NOT NULL                       -- appended column
ALTER TABLE … ENABLE ROW LEVEL SECURITY;
ALTER TABLE … FORCE ROW LEVEL SECURITY;
-- four policies keyed on the JWT claim, one per verb:
CREATE POLICY {t}_tenant_select … USING     (tenant_id = current_setting('request.jwt.claims', true)::json ->> 'tenant_id');
CREATE POLICY {t}_tenant_insert … WITH CHECK (…);
CREATE POLICY {t}_tenant_update … USING (…) WITH CHECK (…);
CREATE POLICY {t}_tenant_delete … USING     (…);
CREATE INDEX IF NOT EXISTS {t}_tenant_idx ON … (tenant_id);
GRANT SELECT, INSERT, UPDATE, DELETE ON … TO authenticated;
```

Policy statements are emitted inside existence-guarded `DO` blocks, so every
plan is replay-safe.

---

## `POST /schema/v1/plan`

Computes the DDL and a diff against the live schema. **Executes no DDL and
touches no user schema** — its only write is one durable `planned` row in the
`flint_schema.provision_ledger` plan store (reused when you re-plan the same
content).

### Example — plan a table

```bash
source .env.keys   # provides FLINT_SERVICE_ROLE_KEY (server-side only!)

curl -sS -X POST "$FORGE_URL/schema/v1/plan" \
  -H "Authorization: Bearer $FLINT_SERVICE_ROLE_KEY" \
  -H "content-type: application/json" \
  -d @permit-watch.spec.json | tee plan.json | jq '{planId, planHash, noop, warnings}'
```

Response `200`:

```jsonc
{
  "planId": "pln_7c3f0e…",
  "planHash": "sha256:9f2c…",       // covers the spec AND the generated DDL
  "namespace": "acme_sourcing",
  "operations": [
    { "kind": "create_table",  "target": "acme_sourcing.permit_watch", "exists": false },
    { "kind": "enable_rls",    "target": "acme_sourcing.permit_watch", "exists": false },
    { "kind": "create_policy", "target": "permit_watch_tenant_select", "exists": false },
    // … insert/update/delete policies, tenant index, grant, caller indexes
  ],
  "ddl": "-- generated by FFS-001 schema provisioning (p17); additive-only, replay-safe\n\nCREATE TABLE …",
  "warnings": [],
  "noop": false,                     // true ⇒ live schema already satisfies the spec
  "expiresAt": "2026-07-31T10:00:00Z" // plans expire 24h after creation
}
```

**Read the `ddl` field before applying — that review is the point of the
two-step design.** In CI, commit `plan.json` to the PR so a human approves
the exact statements.

| Error | Meaning |
|---|---|
| `400 invalid spec: …` | Parse or validation failure; the message names the offending element |
| `403 namespace is reserved …` | `flint_*` / `public` / `pg_*` / `information_schema` |
| `403 namespace is not in the provisioning allowlist` | Operator has not allowlisted it |
| `409 namespace schema does not exist …` | Operator must create + grant the schema first (runbook §14) |

## `POST /schema/v1/apply`

```bash
curl -sS -X POST "$FORGE_URL/schema/v1/apply" \
  -H "Authorization: Bearer $FLINT_SERVICE_ROLE_KEY" \
  -H "content-type: application/json" \
  -d "{\"planHash\": \"$(jq -r .planHash plan.json)\"}" | jq
```

Apply loads the stored spec by hash, **re-plans it against the live schema,
and refuses if the recomputed hash differs** — the drift guard. Because the
hash covers the generated DDL (not just the spec), any out-of-band schema
change between plan and apply changes the recomputed hash and produces a
`409`. Execution is one explicit transaction, committed, run as the
dedicated `flint_provisioner` Postgres role, attributed in the ledger to the
caller's JWT `sub`.

Response `200`:

```jsonc
{
  "planId": "pln_7c3f0e…",
  "applied": true,
  "alreadyApplied": false,          // true on same-hash replay (idempotent, nothing re-executed)
  "schemaVersionBefore": 41,
  "schemaVersionAfter": 42,
  "reflectionRefreshed": true,
  "restartRequired": true,
  "restartNote": "REST routes for new tables are mounted at startup; OpenAPI, MCP tools, GraphQL subscriptions and the A2UI catalog are live now."
}
```

**`restartRequired: true` is honest, not boilerplate.** The reflection
recompile makes the new table visible immediately in `/openapi.json`, MCP
tools, GraphQL subscriptions and the A2UI catalog — but per-table REST routes
(`/{schema}/{table}`) are mounted at gateway startup. Restart the gateway to
get them.

| Error | Meaning |
|---|---|
| `404 unknown planHash` | Never planned (or ledger row removed) |
| `410 plan expired …` | Older than 24h — re-plan and re-review |
| `409 plan no longer matches live schema` | Drift since plan time — re-plan |
| `409 previous apply of this plan failed …` | Inspect the ledger, create a new plan |
| `403` | Namespace no longer allowlisted / reserved (apply honors the operator's *current* configuration, even for replay) |
| `500` | DDL failed: transaction rolled back, ledger row `failed` with the Postgres SQLSTATE (never the statement) |

## `GET /schema/v1/status`

```bash
curl -sS "$FORGE_URL/schema/v1/status" \
  -H "Authorization: Bearer $FLINT_SERVICE_ROLE_KEY" | jq
```

```jsonc
{
  "enabled": true,
  "namespaces": ["acme_sourcing", "acme_reports"],
  "schemaVersion": 42,
  "lastApply": { "planId": "pln_7c3f0e…", "at": "2026-07-30T17:12:00Z", "status": "applied" }
}
```

Works before any provisioning has ever happened (`lastApply: null`). Useful
as the deploy-time readiness probe for provisioning pipelines.

## `GET /schema/v1/tables/{schema}/{table}/ddl`

Synthesizes a `CREATE TABLE` statement for an **existing** table — the piece
that lets a client drive `registerEntityFromSql`-style integrations at
runtime.

```bash
curl -sS "$FORGE_URL/schema/v1/tables/acme_sourcing/permit_watch/ddl" \
  -H "Authorization: Bearer $FLINT_SERVICE_ROLE_KEY" | jq -r .ddl
```

```sql
CREATE TABLE "permit_watch" (
  "id" uuid NOT NULL DEFAULT gen_random_uuid(),
  "api_number" text NOT NULL,
  "filed_at" timestamp with time zone,
  "payload" jsonb NOT NULL DEFAULT '{}'::jsonb,
  "tenant_id" text NOT NULL,
  PRIMARY KEY ("id")
);
```

Full response carries `rlsEnabled`, `rlsForced`, and `schemaVersion`.
Identifiers are quoted (tables created out of band may carry any name);
composite primary keys render in key order, not column order. Types render
in Postgres's canonical spellings (`timestamp with time zone`, not
`timestamptz`).

| Error | Meaning |
|---|---|
| `400 invalid schema or table identifier` | Injection-shaped path segment, rejected before any query |
| `404 unknown table` | No such table in that schema |

This endpoint requires `service_role` like the rest of the group: a full
column inventory is a better reconnaissance target than the lossy JSON
Schema in `/openapi.json`.

---

## The audit ledger

Every provisioning action lands in `flint_schema.provision_ledger`
(migration `0015`):

| Column | Content |
|---|---|
| `plan_id`, `plan_hash`, `namespace`, `spec`, `generated_ddl` | What was planned |
| `status` | `planned` → `applied` \| `failed` (hash-constrained transitions — a failed row can never flip to applied, a reused id can never bless different content) |
| `applied_by` | The caller's JWT `sub` — never the bearer |
| `applied_at`, `version_before`, `version_after` | When, and the reflection schema versions around it |
| `error_code` | Postgres SQLSTATE only — never the statement text |

DDL cannot commit without its ledger transition: an apply whose planned row
is missing rolls the whole transaction back.

## Recommended workflow (CI/CD)

FFS-001 §12's "Option A": **a manifest declares; a pipeline provisions; a
human reviews in between.** A manifest never calls this API at runtime.

```yaml
# Build stage — generate the plan and surface it in the PR
- run: |
    curl -sS -X POST "$FORGE_URL/schema/v1/plan" \
      -H "Authorization: Bearer $FLINT_SERVICE_ROLE_KEY" \
      -H "content-type: application/json" \
      -d @entities.spec.json > plan.json
    jq -r .ddl plan.json          # goes into the PR for review

# Deploy stage — apply the reviewed plan (idempotent; safe on redeploys)
- run: |
    HASH=$(jq -r .planHash plan.json)
    curl -sS --fail-with-body -X POST "$FORGE_URL/schema/v1/apply" \
      -H "Authorization: Bearer $FLINT_SERVICE_ROLE_KEY" \
      -H "content-type: application/json" \
      -d "{\"planHash\":\"$HASH\"}"
```

A `409` at deploy time means the schema moved between review and deploy —
that is the guard working; re-plan, re-review, redeploy.

## Best practices

- **Review `ddl` before every apply.** The plan step exists so a human (or a
  strict CI check) sees the exact statements. Never pipe plan→apply blindly.
- **Keep the `service_role` key server-side, always.** It carries standing
  DDL rights; treat it like a database password. The `anon` key must be
  refused by this API (`403`) — that refusal is covered by tests, but don't
  rely on it as your only line: never ship `service_role` to a client.
- **One namespace per application module** keeps grants reviewable and blast
  radii small; a shared namespace is cheaper but weaker isolation. This is
  an application decision — Forge supports both.
- **Prefer `tenantScoped: true` for everything.** The generated RLS block is
  the whole point; `acknowledgeUnscoped` exists for genuinely tenant-free
  reference data, and such tables are invisible to the reflection surfaces
  until you add RLS yourself.
- **Treat `noop: true` as success** in pipelines — it means the declared
  state already holds.
- **Don't renumber or reuse plan hashes.** Replay of an applied hash is a
  safe no-op (`alreadyApplied`); a failed plan wants a *new* plan after you
  fix the cause, not a retry of the old hash.
- **Destructive changes go through `migrations/`.** DROP/RENAME/type changes
  are deliberately impossible here; the API refusing a drifted column with a
  pointer at migrations is by design.
- **After apply, restart the gateway when you need the REST routes** —
  everything else is live immediately.
