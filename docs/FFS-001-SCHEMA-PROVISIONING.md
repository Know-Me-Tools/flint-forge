# FFS-001 — Schema Provisioning API: As-Built Specification & Decision Record

**Status:** SHIPPED (p17, merged via PR #31, 2026-07-30)
**API reference:** [`docs/api/schema-provisioning.md`](api/schema-provisioning.md)
**Operator guide:** [`docs/runbook.md` §14](runbook.md)
**Credentials:** [`docs/ANON-SERVICE-ROLE-KEYS.md`](ANON-SERVICE-ROLE-KEYS.md)
**Phase audit trail:** `.kbd-orchestrator/phases/p17-schema-provisioning/` (assessment, plan, execution ledger, six-run boundary record, reflection) and `.kbd-orchestrator/changes/archive/2026-07-30-p17-*/verification.md`

This document records **why** the `/schema/v1` API is shaped the way it is:
the eight design decisions it was built on, and — just as important — the
four places where the original proposal was **wrong** and reality forced a
correction. Read this before extending the API.

---

## 1. The problem it solves

Forge is a *reflect-what-exists* gateway: it detects DDL (via the
`flint_meta` event trigger → `pg_notify('meta_runtime')` → `StateManager`
hot-swap) but never authored it. Before p17, an application that needed a new
entity required a hand-written migration in a different repo, applied by a
different person, with tenant RLS re-authored by hand each time —
`migrations/0013_force_rls.sql` explicitly disclaimed operator tables:
*"operators MUST apply FORCE ROW LEVEL SECURITY to their own RLS-governed
tables."* Hand-authored RLS-per-table is exactly where isolation bugs come
from. FFS-001 closes that gap by making tenant RLS **generated and
unforgettable**.

## 2. Design decisions (D1–D8) and their reasoning

### D1 — Structured spec only; no raw DDL at any privilege level
The API accepts a typed JSON spec and generates SQL. A filtered DDL string is
a parser battle you must win every time; a closed grammar is a battle you win
once. Column types are a closed Rust enum, every identifier passes
`forge_domain::is_safe_identifier`, defaults come from a strict allowlist —
so injection fails at deserialization/validation before any code runs. An
`"escapeHatch": "raw_sql"` field was considered and rejected: once it exists,
every caller uses it and the guarantee is gone. Advanced cases go through
`migrations/`.

### D2 — Plan / apply, not a single mutating call
DDL is close to irreversible. A plan is reviewable by a human or CI, diffable
in a PR, and content-hashed so apply is idempotent (replay = no-op, not a
second `ALTER`). It also lets clients plan at build time and apply at deploy
time — where provisioning belongs.

### D3 — Caller authority ≠ database authority
The caller proves `service_role` via JWT; the DDL executes as a dedicated
`flint_provisioner` Postgres role (`NOLOGIN`, no `BYPASSRLS`, `CREATE` only
on operator-granted namespaces, no database-level `CREATE`). The
`service_role` credential is a 10-year key, so token lifetime contributes
nothing to containment — the Postgres role boundary and the namespace
allowlist are the *only* things between a leaked key and the database.
Design as if the key is already public. Verified by test: a `CREATE` in an
ungranted namespace fails with SQLSTATE `42501` at the Postgres layer, no
matter what the application layer would allow.

### D4 — Namespace allowlist, operator-controlled, default off
`FLINT_PROVISION_NAMESPACES` empty/unset ⇒ every endpoint returns `503` and
no DDL path exists. This ships into existing deployments; an operator who has
not opted in must not gain a DDL surface by upgrading. `flint_*`, `public`,
`pg_*`, `information_schema` are refused unconditionally, before the
allowlist. **As-built refinement:** the routes are *always mounted* and the
gate lives in the handlers — the proposal's "feature-gate mounting" wording
would have produced `404` where the contract requires `503`.

### D5 — Tenant scoping is generated, never requested
`tenantScoped: true` produces, verbatim and unconditionally: the `tenant_id`
column, `ENABLE` + `FORCE ROW LEVEL SECURITY`, four fixed per-verb policies
keyed on `request.jwt.claims ->> 'tenant_id'`, a tenant index, and CRUD
grants to `authenticated`. The caller cannot supply, override, or disable a
policy body. The most common multi-tenant bug is a table that ships without
RLS because someone forgot; making that impossible is most of the value of
the feature. Proven end-to-end: the two-tenant isolation test provisions a
table through the real API and asserts tenant B sees nothing and cannot
forge tenant A's `tenant_id` on insert.

### D6 — Additive-only in v1
`CREATE TABLE`, guarded `ADD COLUMN` (nullable or defaulted), `CREATE INDEX`,
policies + grants. No `DROP`, no `RENAME`, no type changes — those need a
review workflow this API deliberately does not have. **As-built refinement:**
an existing column whose type or nullability differs from the spec is
*refused* (`ColumnDrift`) rather than silently counted as satisfied — a
false "noop" would hide real divergence.

### D7 — Apply must commit explicitly, on its own pool
`PgBackend::acquire` opens a connection-lifetime transaction (required for
`SET LOCAL` RLS context); an uncommitted write there is silently rolled back
on pool recycle while `RETURNING` still reports success. For DDL that
failure mode is a `200 OK` and no table. The provisioning adapter therefore
owns its own pool (`PROVISIONER_DATABASE_URL`) and its own explicit
`BEGIN … COMMIT`. The regression test asserts the applied table is visible
on a *fresh connection*. The commit is additionally conditional on the
ledger transition matching exactly one `planned` row with the right hash —
DDL can never land without its audit record (Base Rule #18).

### D8 — Honest disclosure of the route-mount gap
On apply, reflection recompiles immediately: the new table is live in
`/openapi.json`, MCP tools, GraphQL subscriptions, and the A2UI catalog
(verified by the hot-swap e2e — `openapi_handler` reads
`state_manager.current()` per request). Per-table REST routes are mounted at
startup, so every apply response carries `restartRequired: true` with an
explanatory note. The catch-all delegate that would flip it to `false` is
specced (p17-c007) behind a numeric benchmark gate and was deliberately not
shipped unbenchmarked: an honest flag beats a slow gateway.

## 3. Where the proposal was wrong — corrections that shipped

These four items are the most instructive part of the record: each passed
document review and survived until either implementation or the live
phase-boundary test run disagreed.

| # | Proposal said | Reality | As-built correction |
|---|---|---|---|
| 1 | §5's migration SQL verbatim: `FORCE ROW LEVEL SECURITY` on `provision_ledger`, grants to `flint_provisioner` | FORCE RLS with **zero policies** is default-deny for every non-BYPASSRLS role — the grants were dead letters; every ledger write would have failed | Migration `0015` adds a provisioner-only policy (`provision_ledger_provisioner_all`); `authenticated`/`anon` still have no path in; `service_role` passes via real `BYPASSRLS` (0014) |
| 2 | §4.2: plans include a `create_schema` operation | `CREATE SCHEMA IF NOT EXISTS` requires database-level `CREATE` **even when the schema already exists** (SQLSTATE `42501`) — and D3 forbids granting it. The spec's own §10 has the operator creating schemas | The generator never emits `CREATE SCHEMA`; plan/apply gate on a `schema_exists()` port check and return an actionable `409` pointing at runbook §14. Found by the first live boundary run, after passing every static review |
| 3 | §4.3: "re-plans from the stored spec and refuses if the recomputed hash differs — that is the drift guard" — with the hash defined over the spec | A spec-only hash trivially equals its own re-plan; the guard as written could never fire | The `planHash` covers the canonicalized spec **and the generated DDL**, which is a function of `(spec, live)` — so live drift changes the recomputed hash. Regression test: `hash_detects_live_schema_drift` |
| 4 | §9: "rotate and confirm the old token dies" framed as a simple assertion | The JWKS cache is process-global with a 600s default TTL and is not URL-keyed: a rotated-out key keeps verifying on a warm gateway until the TTL expires | Measured by `rotation_revocation.rs`; runbook §14 states the operational rule — incident-grade revocation = rotate **and restart** (or run a lower `FLINT_GATE_JWKS_TTL_SECS`) |

The general lesson, recorded in the phase reflection: *a spec's evidence
table being verifiably accurate does not make its artifacts executable* —
claims were checked and correct while two of its SQL/API shapes were
production bugs. Every reviewer (spec author, implementer, adversarial
judge) shared the spec's frame; only executing against live Postgres broke
it.

## 4. Architecture placement

Hexagonal, matching the workspace rule (domain/app never import adapters;
composition only in the interface crate):

| Crate | Adds |
|---|---|
| `fdb-domain` | `provision/` — `SchemaSpec`/`TableSpec`/`ColumnSpec`, closed `ColumnType`, `IndexSpec`, transparent `PlanId`/`PlanHash`/`Namespace`, `Plan`/`Operation`, plan-store DTOs, and the validation pass |
| `fdb-app` | `provision/` — pure deterministic `generate(spec, live) → Plan` (the highest-risk code, unit-testable with no database), canonical hashing, `synthesize_create_table` |
| `fdb-ports` | `SchemaProvisioner` — the only DDL-adjacent seam; sits beside `SchemaProvider` (which introspects; this one mutates) |
| `fdb-postgres` | `PgProvisioner` — own pool, explicit transactions, `SET LOCAL ROLE flint_provisioner`, pg_catalog introspection, SQLSTATE-only errors |
| `fdb-gateway` | `schema_api` (on the **library** target so integration tests drive the real handlers), mounted in `bootstrap.rs` |

## 5. Verification summary

Phase-boundary record (six runs, final): **611 passed / 0 failed / 6
ignored** — full workspace against a live `flint-forge-pg:18` container with
migrations + seed, a served JWKS, and the real `anon`/`service_role` keys.
Highlights: full injection corpus; D7 fresh-connection commit; failure
rollback with SQLSTATE-only errors; `42501` privilege containment;
401/401/403/503/409/404/410 route matrix with real credentials
(`anon` → `403`); two-tenant isolation on a provisioned table; 36-combination
DDL round-trip; OpenAPI hot-swap without restart; rotation revocation
simulation.

## 6. Deliberately out of scope (and why)

- **Destructive operations** — need a review workflow this API refuses to
  have; use `migrations/`.
- **Route hot-reload** (p17-c007) — specced with a numeric ship gate
  (mounted-route p99 regression <5%, delegated overhead <1ms p99); not
  shipped unbenchmarked.
- **Short-lived provisioning tokens** (proposal §11.7) — open; today the
  allowlist + role split bound the standing grant.
- **Cross-tenant or cross-namespace provisioning in one call** — never.
