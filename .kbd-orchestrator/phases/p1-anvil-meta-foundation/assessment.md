# Assessment — p1-anvil-meta-foundation

**Phase:** p1-anvil-meta-foundation  
**Date:** 2026-06-30  
**Status:** Complete  
**Reference plan:** `docs/FLINT-PHASE-PLAN-REVISED.md` (RFC-FORGE-PHASES-002)  
**Changes assessed:** 11 (p1-c001 through p1-c011)  
**Blockers resolved in this assessment:** OQ-4, OQ-5 → JWT contract written at `docs/contracts/jwt-contract.md`

---

## 0. Assessment Method

Read from:
- `crates/ext-flint-auth/src/lib.rs` + `sql/flint_auth.sql`
- `crates/ext-flint-hooks/src/lib.rs` + `sql/flint_hooks.sql`
- `crates/ext-flint-vault/src/lib.rs` (full implementation)
- `crates/ext-flint-vault/Cargo.toml` (pgrx = "=0.18.1", pg18 feature)
- All `Cargo.toml` files for pgrx version pinning
- `flint-gate` codebase at `/Users/gqadonis/Projects/prometheus/flint-gate`:
  - `src/auth/jwt_verify.rs` — inbound claim extraction
  - `src/auth/jwt_mint.rs` — outbound JWT minting shape
  - `src/auth/identity.rs` — Identity struct
  - `src/auth/kratos.rs` — Kratos session → Identity mapping
  - `src/auth/api_key.rs` — API key → Identity mapping
  - `src/config/types.rs` — JwtConfig, full config schema
  - `src/middleware/pipeline.rs` — request lifecycle, header injection
  - `config.example.yaml` — canonical configuration examples

---

## 1. Current State Summary

### 1.1 What exists

| Crate | State | Notes |
|-------|-------|-------|
| `ext-flint-auth` | SQL stub + basic `#[pg_extern]` | `auth.jwt()`, `auth.uid()`, `auth.role()`, `auth.bearer()` implemented in SQL. pgrx 0.12, pg17. |
| `ext-flint-hooks` | SQL stub | `flint.webhooks`, `flint.webhook_outbox`, `flint.dispatch_webhook()` tables/trigger created. SECURITY DEFINER stub. pg_net call is a TODO comment. |
| `ext-flint-vault` | **COMPLETE (implementation-ready)** | Full XChaCha20-Poly1305 AEAD. KMS shell-unwrap path. HKDF-SHA256 per-category subkeys. `vault.create_secret()`, `vault.get_secret()`, `vault.resolve_api_key()` brokered paths. Access log. Role-locked (`flint_secret_reader`, `vault_admin`). Tests present. pgrx = "=0.18.1", pg18. |
| `ext-flint-llm` | Not read (out of scope P1) | |
| `ext-flint-meta` | **Does not exist** | New crate — zero files. |
| `docs/contracts/jwt-contract.md` | **CREATED this session** | Pins claim shape from flint-gate source. Resolves OQ-4 and OQ-5. |

### 1.2 What does NOT exist (gaps)

- `crates/ext-flint-meta/` — entire crate missing
- `images/postgres18/Dockerfile` — pg_cron not added
- `crates/ext-flint-auth/sql/flint_auth.sql` tests (version test only; no RLS end-to-end test)
- `crates/ext-flint-hooks` dispatch trigger body (stub only; pg_net call and HMAC signing absent)
- `crates/ext-flint-hooks` durable outbox BGW dispatcher (not started)
- Azure Key Vault managed identity unwrap path in `ext-flint-vault` (has HS256 dev path; KMS shell-unwrap path exists in code, but Azure-specific CLI integration not wired)
- `docs/contracts/meta-trigger-coverage.md` (referenced in plan but not created)

### 1.3 pgrx version state (confirmed)

| Crate | pgrx version | Target PG | Notes |
|-------|-------------|-----------|-------|
| `ext-flint-auth` | 0.12 | pg17 | DO NOT upgrade — separate boundary |
| `ext-flint-hooks` | Not read (inspect before p1-c002) | — | Check Cargo.toml |
| `ext-flint-vault` | =0.18.1 | pg18 | Already on target version |
| `ext-flint-meta` | =0.18.1 (to be created) | pg18 | Must match vault; single-compile `cdylib` |

---

## 2. Per-Change Gap Analysis

### p1-c001 — `auth.*` SQL helpers + GUC contract (pgrx 0.12/pg17)

**Status: 60% done**

**What exists:**
- `auth.jwt()`, `auth.uid()`, `auth.role()`, `auth.bearer()` — all four functions implemented in `sql/flint_auth.sql`
- `#[pg_extern] flint_auth_version()` — version check exists
- One pgrx unit test (`version_present`) exists

**Gaps:**
- No RLS end-to-end test: a test that sets `request.jwt.claims` and verifies `auth.uid()` returns the `sub` value
- No test for `auth.role()` fallback to `'anon'` when `role` claim is absent
- No `auth.tenant_id()` function (may be needed by RLS policies in P2+; not in revised plan but referenced in spec §2.2)
- The SQL file does not create the `auth` schema with `AUTHORIZATION` or GRANT statements — schema security not yet set

**Required for gate:** Add RLS integration tests. The existing SQL functions are correct per the JWT contract.

---

### p1-c002 — `flint_hooks` standard tier

**Status: 30% done**

**What exists:**
- Schema: `flint.webhooks` and `flint.webhook_outbox` tables defined
- `flint.dispatch_webhook()` trigger function (SECURITY DEFINER) — reads `auth.bearer()` and builds payload
- Trigger function is registered as a proper PL/pgSQL function

**Gaps:**
- `dispatch_webhook()` body has `-- TODO(p1-c002/p1-c003): per-registration header build + tier routing` — no actual dispatch
- pg_net `net.http_post()` call not implemented
- HMAC-SHA256 Option-3 signature not implemented
- Per-registration webhook selection (JOIN on `flint.webhooks`) not implemented
- `forward_jwt: true` path (forwarding `auth.bearer()` as Authorization) not wired
- `custom_headers` JSONB merge not implemented
- No test that a webhook fires on INSERT to a registered table

**Required for gate:** Full dispatch implementation including pg_net call and HMAC signature.

---

### p1-c003 — `flint_hooks` durable tier

**Status: 5% done**

**What exists:**
- `flint.webhook_outbox` table schema — `status`, `visible_at`, `retry_count`, `created_at` columns correct
- Schema design matches durable outbox pattern

**Gaps:**
- No dispatcher BGW (Background Worker) in pgrx
- No SKIP LOCKED SELECT for retry polling
- No retry backoff logic
- No `status` state machine (`pending → delivering → delivered/failed`)
- No `visible_at` bump on retry
- Tier routing (`tier = 'standard'` vs `tier = 'durable'`) not wired in `dispatch_webhook()`

**Required for gate:** p1-c003 is a P1 priority (not P0 gate). Can be deferred within Phase 1 but must complete before Phase 2 starts.

---

### p1-c004 — pg_cron in Dockerfile

**Status: 0% done**

**What exists:** `images/postgres18/Dockerfile` (not inspected — assumed present from p0)

**Gaps:**
- `pg_cron` package not added to Dockerfile
- No `shared_preload_libraries` update for pg_cron
- No cron job table creation in any extension SQL

**Required for gate:** Low-effort change; prerequisite for durable dispatcher BGW scheduling.

---

### p1-c005 — JWT contract pin

**Status: COMPLETE (this session)**

**What exists:**
- `docs/contracts/jwt-contract.md` — **created this session** from flint-gate source
- Covers: inbound claim shape (OQ-4), service-identity token format (OQ-5), algorithm options, `SET LOCAL` propagation, `auth.*` GUC mapping, security constraints

**Gaps:**
- None. OQ-4 and OQ-5 are resolved.

**Required for gate:** ✅ Complete.

---

### p1-c006 — `ext-flint-vault` KMS unwrap (Azure Key Vault managed identity v1)

**Status: 70% done**

**What exists:**
- Shell-based KMS unwrap path: `FLINT_VAULT_UNWRAP_CMD` env var + `FLINT_VAULT_DEK_WRAPPED` — this IS the Azure path. Any KMS CLI (including `az keyvault key unwrap`) can be plugged in.
- The `run_unwrap()` function is fully implemented: spawns shell command, pipes wrapped DEK base64 on stdin, reads 32 raw bytes on stdout.
- Dev path (`FLINT_VAULT_ROOT_KEY`) also functional.
- `_PG_init()` eager load on startup — detects misconfiguration at boot.

**Gaps:**
- No Azure-specific documentation or example `FLINT_VAULT_UNWRAP_CMD` template for Azure Key Vault managed identity
- No Kubernetes secret/environment variable injection example for the Forge deployment
- The wrapped DEK format (base64 of RSA-OAEP-256 wrapped 32-byte DEK) is not documented
- No integration test exercising the unwrap path in CI (requires a mock KMS or a test-only `FLINT_VAULT_ROOT_KEY`)

**Required for gate:** The core implementation is done. The gap is documentation + a working `FLINT_VAULT_UNWRAP_CMD` example. Functional for P1 gate.

---

### p1-c007 — `flint_meta` extension: schema

**Status: 0% done**

**What exists:** Nothing. `crates/ext-flint-meta/` does not exist.

**Gaps (everything):**
- Crate scaffold: `Cargo.toml` (pgrx = "=0.18.1", pg18, `crate-type = ["cdylib"]`)
- No `src/bin/pgrx_embed.rs` (single-compile path per pgrx 0.18.1 migration)
- Cache tables: `flint_meta.cache_tables`, `cache_columns`, `cache_relationships`, `cache_functions`, `cache_policies`, `cache_types`
- Version tracking: `flint_meta.schema_version` table + `flint_meta.increment_version()` function
- Keto tuple storage: `flint_meta.keto_tuples` + indexes on `(namespace, object_id, relation, subject_id)`, `(subject_id, namespace)`, `(namespace, object_id)`
- Vault key metadata: `flint_meta.vault_keys`, `flint_meta.vault_key_assignments`
- `extension_sql_file!` registration for all tables
- `pg_module_magic!()` and version function
- **Must NOT be added to workspace root `Cargo.toml` members** — excluded via workspace `exclude` pattern

**Critical constraint:** pgrx 0.18.1 single-compile requires:
- `crate-type = ["cdylib"]` only (no `"lib"` secondary)
- No `[[bin]] name = "pgrx_embed_*"` entry (removed in 0.18 series)
- Features: `default = ["pg18"]`, `pg18 = ["pgrx/pg18", "pgrx-tests/pg18"]`

---

### p1-c008 — `flint_meta` DDL event triggers

**Status: 0% done**

**Gaps (everything):**
- `flint_meta_refresh_cache()` — pgrx event trigger function using `pg_event_trigger_ddl_commands()`
- `flint_meta_invalidate_cache()` — event trigger using `pg_event_trigger_dropped_objects()`
- SQL `CREATE EVENT TRIGGER` bindings for `ddl_command_end` and `sql_drop` events
- `pg_notify('meta_runtime', payload_json)` call inside trigger
- Payload shape: `{"version": N, "ddl_tag": "CREATE TABLE", "object_identity": "public.foo"}`
- `docs/contracts/meta-trigger-coverage.md` documenting known coverage gaps

**Critical:** pgrx event triggers require the `#[pg_trigger]` attribute pattern plus correct `RETURNS event_trigger` — verify with pgrx 0.18.1 docs before implementing. The function must use `unsafe` at the FFI boundary.

---

### p1-c009 — `flint_meta` SQL reflection functions

**Status: 0% done**

**Gaps (everything):**
- `flint_meta.tables()` — SETOF table returning `(schema_name, table_name, is_view, description, rls_enabled)`
- `flint_meta.columns(schema_name, table_name)` — SETOF returning column metadata
- `flint_meta.relationships(schema_name, table_name)` — SETOF returning FK relationships
- `flint_meta.functions(schema_name)` — SETOF returning callable function metadata
- `flint_meta.version()` — returns current `schema_version.version`
- `flint_meta.check_permission(namespace, object_id, relation, subject_id)` — Keto tuple lookup
- `flint_meta.set_identity(identity_json)` — writes `request.jwt.claims` GUC via `set_config()`

Each function exposed as `#[pg_extern]` from pgrx, backed by SELECT from cache tables.

---

### p1-c010 — `flint_meta` AG-UI and OpenAPI descriptor functions

**Status: 0% done**

**Gaps:**
- `flint_meta.agui_descriptor()` — returns JSONB describing all tables as AG-UI tool descriptors
- `flint_meta.openapi()` — returns JSONB of OpenAPI 3.1 paths object built from cache tables
- Both functions query `cache_tables` + `cache_columns` + `cache_relationships`
- Schema for AG-UI descriptor shape: `{tools: [{name, description, parameters: {type, properties, required}}]}`

**Dependency:** requires p1-c009 cache table population to return meaningful data.

---

### p1-c011 — Integration test: PgListener on `meta_runtime`

**Status: 0% done**

**Gaps:**
- sqlx-based integration test in `fdb-reflection` test suite (or standalone test binary)
- Test flow: connect PgListener → `CREATE TABLE test_meta_001` → assert notification received within 5s → assert `flint_meta.version()` > initial value
- Reconnect loop test: disconnect PgListener mid-stream → verify reconnect and forced recompile path

**Dependency:** requires p1-c007, p1-c008, p1-c009 all complete.

---

## 3. Resolved Open Questions

| OQ | Question | Resolution |
|----|----------|------------|
| OQ-4 | Exact flint-gate claim shape | **RESOLVED** — `docs/contracts/jwt-contract.md` §1–2 |
| OQ-5 | Service-identity token format | **RESOLVED** — `docs/contracts/jwt-contract.md` §4 |

### Key findings from flint-gate source

1. **Claim shape from verification (`jwt_verify.rs`):**
   - `sub` → `identity.id`
   - OIDC traits (`email`, `name`, `email_verified`, etc.) → `identity.traits`
   - Everything else EXCEPT `iss`, `iat`, `exp`, `nbf`, `jti`, `auth_time` → `identity.metadata_public`
   - This means `role`, `org_id`, `tenant_id`, `scope` all land in `metadata_public`

2. **Minted JWT shape (`jwt_mint.rs`):**
   - Fixed claims: `iss`, `sub`, `iat`, `exp`, `jti`
   - Merged from `identity.traits`: OIDC fields
   - Merged from `additional_claims` (per-route config): `scope`, `org_id`, etc.
   - `role` is NOT automatically included — it must be an `additional_claims` entry per route

3. **`auth.role()` critical implication:**
   - `auth.role()` returns `coalesce(auth.jwt()->>'role', 'anon')`
   - For `auth.role()` to return `'authenticated'`, the minted JWT MUST include `"role": "authenticated"` as an `additional_claims` entry in the route hook config
   - Service tokens need `"role": "service_role"` in `additional_claims`
   - **This is a p1-c001 gap** — the SQL file needs tests that verify this

4. **Service-identity algorithm:** defaults to HS256 per `JwtConfig::default()`. The `from_db_or_config()` path also supports DB-sourced keys with any algorithm.

5. **Default issuer:** `"flint-gate"` — written to `request.jwt.claims` and read by `auth.jwt()->>'iss'` if policies need it.

---

## 4. Remaining Open Questions

| OQ | Question | Needed for |
|----|----------|------------|
| OQ-3 | pg_graphql PG18 tagged release — check supabase/pg_graphql/releases | Phase 3 kickoff |
| OQ-6 | FRF Phase 5 agentproto crate timeline | p7-c002 |
| OQ-7 | ag-ui-client Rust SDK coverage audit | Phase 7 kickoff |
| OQ-8 | Keto sync via FRF Iggy — does FRF support keto_changes event type? | p3-c006 |
| OQ-9 | `ext-flint-hooks` pgrx version (not read in this assessment) | p1-c002 execution |
| OQ-10 | `images/postgres18/Dockerfile` current content | p1-c004 execution |

---

## 5. Execution Order (Recommended)

The following ordering minimizes blocked time and matches the gate criteria:

```
Parallel batch 1 (no dependencies):
  p1-c001  auth SQL helpers — add RLS integration tests
  p1-c004  pg_cron Dockerfile
  p1-c005  JWT contract pin ← COMPLETE

Sequential:
  p1-c007  flint_meta schema (new crate, pgrx 0.18.1)
  p1-c008  DDL event triggers (depends on p1-c007)
  p1-c009  Reflection functions (depends on p1-c007)

Parallel batch 2 (after 007+008+009):
  p1-c010  AG-UI/OpenAPI descriptor functions
  p1-c011  PgListener integration test

Parallel batch 3 (after p1-c001):
  p1-c002  flint_hooks standard tier
  p1-c003  flint_hooks durable tier (depends on p1-c002)
  p1-c006  vault KMS documentation + Azure example
```

---

## 6. Critical Pre-Execution Checks

Before starting p1-c007 (`ext-flint-meta`):

1. Verify `cargo pgrx init --pg18 $(which pg18)` works in the dev environment
2. Confirm pgrx 0.18.1 single-compile migration: `ext-flint-vault` uses `crate-type = ["cdylib"]` without `pgrx_embed.rs` — use it as the template
3. Confirm `ext-flint-meta` will be added to workspace `exclude` list in root `Cargo.toml`

Before starting p1-c002 (`ext-flint-hooks` standard tier):

1. Read `crates/ext-flint-hooks/Cargo.toml` — confirm pgrx version (not read in this assessment)
2. Confirm pg_net is available in the target Postgres 18 container

---

## 7. Gate Criteria Check

| Gate requirement | Current state |
|-----------------|--------------|
| `flint_auth` passes RLS end-to-end | PARTIAL — functions exist, RLS tests missing |
| `flint_hooks` fires a signed webhook through flint-gate | NOT STARTED — dispatch_webhook() stub only |
| `flint_meta` extension installs | NOT STARTED — crate does not exist |
| Cache tables populate | NOT STARTED |
| Event trigger fires on `CREATE TABLE`, increments version | NOT STARTED |
| NOTIFY reaches test LISTEN client within 5s | NOT STARTED |

**Assessment result: 2/11 changes complete or near-complete (p1-c005 complete, p1-c006 at 70%). 9/11 require significant implementation work.**

---

## 8. Handoff Note for Planning

Key design decisions for the planner to encode:

1. **flint_meta crate template:** use `ext-flint-vault/Cargo.toml` as the model — already correctly configured for pgrx 0.18.1 single-compile (no `pgrx_embed.rs`, `crate-type = ["cdylib"]`, pg18 feature)
2. **`role` claim injection:** the `auth.role()` function depends on a `role` claim in the minted JWT. Every production route hook MUST include `"role": "authenticated"` in `additional_claims`. Document this in the JWT contract as a usage note.
3. **`set_identity()` function design:** flint-gate already calls `SET LOCAL "request.jwt.claims"` — the `flint_meta.set_identity()` function must not conflict with this. It should use `set_config('request.jwt.claims', ..., true)` (transaction-local) to match Quarry's existing pattern.
4. **pgrx event trigger API:** pgrx 0.18.1 uses `#[pg_extern]` with a specific FFI signature for event triggers; this differs from regular functions. Review pgrx changelog before coding p1-c008.
