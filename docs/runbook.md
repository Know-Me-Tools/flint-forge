# Flint Forge — Operations Runbook

> **Audience:** On-call engineers, SREs, and DevOps operators.
> **Last updated:** 2026-07-06
> **Change control:** Update via PR; tag with `[ops]` in the commit message.

---

## 1. Stack Overview

### 1.1 Service Map

| Service | Binary | Internal name | Port | Protocol |
|---|---|---|---|---|
| Quarry Gateway | `fdb-gateway` | `flint-quarry` | **8080** | HTTP/JSON, GraphQL over HTTP+WS |
| Kiln Server | `fke-server` | `flint-kiln` | **8090** | HTTP/JSON |
| PostgreSQL | `db` | — | **5432** | Postgres wire protocol |
| Keto (optional) | `keto` | — | 4466 | HTTP (relation checks) |
| FRF gRPC (optional) | `frf` | — | 50051 | gRPC (change stream) |

### 1.2 Port Table (host-facing)

```
8080  →  fdb-gateway  (REST/GraphQL/MCP/A2A/AG-UI/HTMX)
8090  →  fke-server   (WASM function invocation, admin)
5432  →  db           (PostgreSQL — do not expose externally in production)
```

### 1.3 Dependency Graph

```
  Client
    │
    ▼
fdb-gateway:8080
  ├── PostgreSQL:5432   (reflection pool, RLS pool, A2UI pool, Keto-sync pool)
  ├── fke-server:8090   (Kiln BGW drains webhook_outbox)
  ├── keto:4466         (relation-check — optional, degrades gracefully)
  └── frf:50051         (realtime change stream — optional, falls back to LISTEN)

fke-server:8090
  └── PostgreSQL:5432   (registry, component store, cedar_policies)
```

### 1.4 Route Summary — fdb-gateway

| Path | Method | Auth | Description |
|---|---|---|---|
| `/healthz` | GET | none | Liveness + schema version |
| `/openapi.json` | GET | none | Compiled OpenAPI 3.1 document |
| `/graphql` | GET/POST | Bearer | GraphQL queries, mutations, WS subscriptions |
| `/rpc/vector` | POST | Bearer | pgvector similarity search |
| `/mcp/v1/tools` | GET | Bearer | MCP tool definitions |
| `/mcp/v1/a2ui` | POST | Bearer | MCP JSON-RPC endpoint |
| `/mcp/v1/a2ui/sse` | GET | Bearer | MCP SSE stream |
| `/a2ui/v1/*` | GET/POST | Bearer | A2UI component registry |
| `/a2a/v1` | POST | Bearer | A2A task handler |
| `/.well-known/agent.json` | GET | none | A2A agent card |
| `/htmx/*` | GET/POST | Bearer | HTMX admin renderer |
| `/agents/v1/*` | GET/POST | Bearer | AG-UI event streaming |
| `/public/<table>` | GET/POST/PATCH/DELETE | Bearer | Reflection-compiled CRUD |
| `/rpc/public/<fn>` | POST | Bearer | Reflection-compiled RPC |

### 1.5 Route Summary — fke-server

| Path | Method | Auth | Description |
|---|---|---|---|
| `/healthz` | GET | none | Liveness + plane mode |
| `/functions/v1/{name}` | ANY | Bearer | Invoke WASM function (latest) |
| `/functions/v1/{name}@{version}` | ANY | Bearer | Invoke WASM function (versioned) |
| `/admin/functions` | POST | Bearer | Register function (control-plane only) |
| `/admin/functions` | GET | Bearer | List registered functions (control-plane only) |

---

## 2. Startup Procedure

### 2.1 Prerequisites

- Docker Compose v2.20+
- Environment file at `.env` or exported variables (see §2.2)
- No other process on ports 8080, 8090, or 5432

### 2.2 Required Environment Variables

```bash
# Minimum set — set in .env or export before `docker compose up`
DATABASE_URL=postgres://flint:changeme@db:5432/flint
JWT_SECRET=<your-hs256-secret-min-32-chars>

# Optional — defaults shown
RUST_LOG=info
KETO_BASE_URL=http://keto:4466
FRF_ENDPOINT=http://frf:50051
FLINT_CHANGE_SOURCE=listen          # "listen" = Postgres LISTEN/NOTIFY; "fabric" = FRF gRPC
FLINT_LISTEN_CAPACITY=1024

# JWKS-based bearer verification (forge-identity::verify_and_build, via fdb-auth) — p16-c005
FLINT_GATE_JWKS_URL=https://gate.example.com/.well-known/jwks.json
FLINT_GATE_ISSUER=https://gate.example.com
FLINT_GATE_AUDIENCE=                # required unless FLINT_GATE_MODE=development (see below)
FLINT_GATE_MODE=production          # default; "development" skips the mandatory-audience check
FLINT_GATE_JWKS_TTL_SECS=600        # JWKS cache TTL before a background refetch (default 10 min)
```

**`FLINT_GATE_MODE` / `FLINT_GATE_AUDIENCE`:** production is the default —
unset, empty, or anything other than exactly `development` requires
`FLINT_GATE_AUDIENCE` to be set, and every bearer-verification call fails
closed (`MissingEnv("FLINT_GATE_AUDIENCE")`) until it is. Set
`FLINT_GATE_MODE=development` for local iteration against a gate that hasn't
configured an audience yet.

**`FLINT_GATE_JWKS_TTL_SECS`:** the JWKS cache refreshes automatically once an
entry is older than this TTL, and separately refetches immediately (rate-limited
to once per 5 seconds) whenever a token's `kid` isn't found in the cached set —
so an upstream signing-key rotation is picked up without a gateway restart.

### 2.3 Step-by-Step Startup

```bash
# 1. Start the database first and wait for it to accept connections
docker compose up -d db
docker compose exec db pg_isready -U flint -d flint
# Expected: "localhost:5432 - accepting connections"
# If not ready, wait 5–10 seconds and retry.

# 2. Start both application services
docker compose up -d fdb-gateway fke-server

# 3. Verify gateway health (includes schema version)
curl -sf http://localhost:8080/healthz | jq .
# Expected: {"status":"ok","service":"flint-quarry","schema_version":<n>}

# 4. Verify Kiln health
curl -sf http://localhost:8090/healthz | jq .
# Expected: {"status":"ok","service":"flint-kiln","plane":"data"}

# 5. Verify migrations applied (optional spot-check)
docker compose exec db psql -U flint -d flint \
  -c "SELECT version, description, installed_on \
      FROM _sqlx_migrations ORDER BY installed_on DESC LIMIT 5;"
```

### 2.4 Full Stack Teardown

```bash
# Stop services but retain volumes (data survives)
docker compose down

# Stop services AND wipe all data (destructive — confirm before running)
docker compose down -v
```

---

## 3. Common Errors and Remediation

---

### Error 1 — Gateway fails to start: `thread 'main' panicked at 'reflection pool connect'`

**Symptom**

```
thread 'main' panicked at 'reflection pool connect: …'
# or
Error: Connection refused (os error 111)
```

Gateway exits immediately on startup. `/healthz` is unreachable.

**Diagnosis**

```bash
# Check that the database container is running
docker compose ps db

# Check that Postgres is accepting connections
docker compose exec db pg_isready -U flint -d flint

# Check DATABASE_URL is set and reachable from the gateway container
docker compose exec fdb-gateway env | grep DATABASE_URL
```

**Remediation**

1. Confirm `db` is healthy before starting `fdb-gateway`:
   ```bash
   docker compose up -d db
   # Wait for pg_isready, then:
   docker compose up -d fdb-gateway
   ```
2. If `DATABASE_URL` is wrong (wrong host, user, password, or db name), update `.env` and restart:
   ```bash
   docker compose down fdb-gateway
   docker compose up -d fdb-gateway
   ```
3. If the database schema does not yet exist, run a `db` container first and let gateway apply migrations on startup.

---

### Error 2 — Migration failed on startup: `thread 'main' panicked at 'database migration failed'`

**Symptom**

```
thread 'main' panicked at 'database migration failed: …'
```

Gateway exits after connecting to the database but before serving requests.

**Diagnosis**

```bash
# Inspect gateway logs for the failing migration
docker compose logs fdb-gateway 2>&1 | grep -E "migration|error|panicked"

# Check which migrations have already been applied
docker compose exec db psql -U flint -d flint \
  -c "SELECT version, description, installed_on, success \
      FROM _sqlx_migrations ORDER BY version;"

# Look for the failing migration file (version numbers in migrations/)
ls migrations/
```

**Known issue — duplicate version prefix:** The repository contains two files sharing the
`0005_` prefix (`0005_cedar_policies.sql`, `0005_flint_a2ui_hybrid_search.sql`) and two
files sharing `0006_` (`0006_change_notify.sql`, `0006_flint_a2ui_application_model.sql`).
If sqlx reports a checksum or ordering conflict, the operator must resolve the conflict by
renaming one file and rebuilding the gateway image.

**Remediation**

1. For a transient error (connection dropped mid-migration):
   ```bash
   docker compose restart fdb-gateway
   # sqlx::migrate! is idempotent — already-applied migrations are skipped
   ```
2. For a bad migration file (syntax error, constraint violation):
   - Fix the `.sql` file in `migrations/`.
   - Rebuild and restart the gateway image.
   - If partial state was written, restore from a pre-migration DB snapshot (see §5).
3. To manually inspect migration state:
   ```bash
   docker compose exec db psql -U flint -d flint \
     -c "SELECT * FROM _sqlx_migrations WHERE success = false;"
   ```

---

### Error 3 — All requests return `401 Unauthorized`

**Symptom**

Every authenticated endpoint returns:
```json
{"error": "missing Authorization header"}
# or
{"error": "invalid or expired token"}
# or (GraphQL)
{"errors": [{"message": "invalid or expired token"}]}
```

Even requests with a valid-looking token are rejected.

**Diagnosis**

```bash
# Check that JWT_SECRET is set in the gateway container
docker compose exec fdb-gateway env | grep -i jwt

# Decode the token (without verification) to inspect claims
# Install jwt-cli or use: echo "<base64.payload.sig>" | cut -d. -f2 | base64 -d | jq .

# Check gateway logs for bearer verification failures
docker compose logs fdb-gateway 2>&1 | grep "bearer verification failed"
```

**Remediation**

1. **Missing secret:** Set `JWT_SECRET` in `.env`, then restart gateway:
   ```bash
   docker compose down fdb-gateway
   docker compose up -d fdb-gateway
   ```
2. **Expired token:** Generate a fresh token using the same secret and algorithm.
   The gateway uses HS256 by default via `fdb_auth::rls_from_bearer`.
3. **Wrong algorithm or audience:** Inspect the token's header claim and confirm the
   issuer/audience matches what `fdb_auth` expects. Check `fdb_auth` crate config.
4. **Clock skew:** Ensure the server clock is synchronized (NTP). Token `exp` validation
   fails if the gateway clock is ahead of the token's `iat`.

---

### Error 4 — Kiln function invocation fails with `404 / "not found"`

**Symptom**

```json
{"error": "function my-fn@latest not found"}
# or
{"error": "artifact not found"}
```

`POST /functions/v1/{name}` returns 404.

**Diagnosis**

```bash
# Check the functions registered in the database
docker compose exec db psql -U flint -d flint \
  -c "SELECT id, name, version, active, content_digest \
      FROM flint_kiln.functions ORDER BY name;"

# Check that the WASM artifact exists in the component store
docker compose exec db psql -U flint -d flint \
  -c "SELECT id, digest, length(data) AS bytes \
      FROM flint_kiln.wasm_artifacts ORDER BY id DESC LIMIT 10;"

# Verify Kiln registry (via admin endpoint — requires control-plane build)
curl -sf http://localhost:8090/admin/functions | jq .
```

**Remediation**

1. **Function not registered:** Register the function via the control-plane endpoint:
   ```bash
   curl -X POST http://localhost:8090/admin/functions \
     -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d '{
       "name": "my-fn",
       "version": "1.0.0",
       "manifest": {"capabilities": []},
       "wasm_base64": "<base64-encoded WASM bytes>"
     }'
   ```
2. **Artifact missing from store:** The function manifest references a `content_digest`
   not present in `flint_kiln.wasm_artifacts`. Re-register the function with valid WASM bytes.
3. **Function marked inactive:** Set `active = true` in `flint_kiln.functions` for the
   affected function, then invoke again.

---

### Error 5 — Cedar policy denied on Kiln invocation

**Symptom**

```json
{"error": "invocation error", "details": "policy denied"}
# or HTTP 403 with Cedar deny reason in logs
```

Kiln invocation returns 500/403; gateway logs show Cedar PEP denial.

**Diagnosis**

```bash
# Check active Cedar policies for the Kiln PEP
docker compose exec db psql -U flint -d flint \
  -c "SELECT id, policy_text, enabled \
      FROM flint_kiln.cedar_policies WHERE enabled = true;"

# Check gateway-side Cedar policies (used by the reflection router)
docker compose exec db psql -U flint -d flint \
  -c "SELECT id, policy_text, enabled \
      FROM flint_meta.cedar_policies WHERE enabled = true;"

# Check Kiln server logs for the denied principal/action/resource
docker compose logs fke-server 2>&1 | grep -E "denied|cedar|policy"
```

**Remediation**

1. Add a `permit` rule for the principal/action/resource triple in `flint_kiln.cedar_policies`:
   ```sql
   INSERT INTO flint_kiln.cedar_policies (policy_text, enabled)
   VALUES (
     'permit(principal == User::"user@example.com",
             action == Action::"invoke",
             resource == Function::"my-fn");',
     true
   );
   ```
2. The Cedar engine hot-reloads from the database — no restart required. Wait ~5 seconds
   for the policy cache to refresh, then retry the invocation.
3. For a blanket allow (dev/staging only — **never in production**):
   ```sql
   INSERT INTO flint_kiln.cedar_policies (policy_text, enabled)
   VALUES ('permit(principal, action, resource);', true);
   ```
4. If the policy source is unreachable (DB down), the Cedar engine starts in `deny-all`
   mode (`SourceUnavailable`). Resolve the DB connectivity issue first.

---

### Error 6 — Rate limit `429` flooding logs

**Symptom**

Gateway or Kiln logs show a flood of 429 responses. Log lines like:
```
WARN rate limit exceeded source=1.2.3.4 path=/functions/v1/my-fn
```

Clients report intermittent `429 Too Many Requests`.

**Diagnosis**

```bash
# Check request volume by source IP (requires access logs or a metrics endpoint)
docker compose logs fdb-gateway 2>&1 | grep "429" | awk '{print $NF}' | sort | uniq -c | sort -rn | head

# Check if it is a single client or distributed
docker compose logs fke-server 2>&1 | grep "rate limit" | tail -50

# Confirm the pattern: steady ramp (legitimate load) vs sudden burst (attack/misconfigured client)
```

**Remediation — legitimate traffic spike**

1. Increase rate limit ceiling in the compose environment:
   ```bash
   # Set in .env and restart affected service
   RATE_LIMIT_RPS=500          # requests per second per IP
   RATE_LIMIT_BURST=1000
   docker compose restart fdb-gateway
   ```
2. Scale horizontally behind a load balancer if the rate increase is sustained.

**Remediation — suspected attack**

1. Identify the attacking IP(s) from logs (see diagnosis above).
2. Block at the network layer (firewall, cloud WAF, or nginx `deny`).
3. Do **not** raise rate limits under attack — that defeats the protection.
4. Alert the security contact (see §7) if the pattern looks like a DDoS or credential-stuffing attempt.

---

## 4. Migration Procedure

All migrations live in `migrations/` at the workspace root and are embedded into the
`fdb-gateway` binary at compile time via `sqlx::migrate!("../../migrations")`.
The migrator is **idempotent** — applied migrations are checksummed and skipped on
subsequent runs. Startup is aborted if any migration fails.

### 4.1 Current Migration Files

```
migrations/
  0002_flint_a2ui.sql
  0003_a2ui_triggers.sql
  0004_flint_a2ui_sdk_extensions.sql
  0005_cedar_policies.sql
  0005_flint_a2ui_hybrid_search.sql
  0006_change_notify.sql
  0006_flint_a2ui_application_model.sql
  0007_flint_a2ui_design_systems.sql
  0008_flint_kiln.sql
  0009_flint_kiln_cedar_policies.sql
```

> **Note:** `0005_` and `0006_` version prefixes each appear on two files. Confirm
> that your sqlx version handles this ordering correctly. If a checksum error occurs,
> rename to unique sequential numbers and rebuild the gateway image.

### 4.2 Apply Migrations

Migrations apply automatically on `fdb-gateway` restart. There is no separate apply step.

```bash
# Force re-apply by restarting the gateway (migrations are idempotent)
docker compose restart fdb-gateway

# Confirm migrations ran
docker compose logs fdb-gateway 2>&1 | grep "migrations applied"
# Expected: INFO database migrations applied
```

### 4.3 Row-Level Security on Tenant Tables (operator responsibility)

Flint Forge's own migrations (`migrations/0013_force_rls.sql`) apply
`FORCE ROW LEVEL SECURITY` to its internal tables (`flint_a2ui.*`,
`flint_kiln.*`). Tenant/application tables — anything an operator creates in
their own schema and exposes via the REST/GraphQL reflection compiler — are
**not** owned by these migrations. When creating a table with an RLS policy,
always apply both statements, not just the first:

```sql
ALTER TABLE public.my_table ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.my_table FORCE ROW LEVEL SECURITY;
```

`ENABLE` alone does not apply RLS to the table's owner or to a superuser
session. Every REST/GraphQL request runs under `SET LOCAL ROLE authenticated`
(a non-owner, non-superuser role — see `fdb-postgres::PgBackend::acquire`),
so `FORCE` is defense-in-depth rather than the primary enforcement mechanism —
but it is what keeps RLS holding even if a future connection somehow acquires
a session without that role de-escalation.

### 4.3 View Applied Migrations

```bash
docker compose exec db psql -U flint -d flint \
  -c "SELECT version, description, installed_on FROM _sqlx_migrations ORDER BY installed_on DESC LIMIT 5;"
```

### 4.4 Diagnose a Failed Migration

```bash
# See all migrations with success flag
docker compose exec db psql -U flint -d flint \
  -c "SELECT version, description, success, installed_on FROM _sqlx_migrations ORDER BY version;"

# Inspect the raw error in gateway logs
docker compose logs fdb-gateway 2>&1 | grep -A 10 "database migration failed"
```

### 4.5 Rollback (No Down Migrations — Snapshot-Based)

sqlx does not generate down migrations for this project. Rollback is performed by
restoring the database volume from a pre-migration snapshot.

```bash
# 1. Stop services
docker compose down

# 2. Wipe the current data volume
docker volume rm flint-forge_postgres_data

# 3. Restore the volume from your backup (example using docker volume import or pg_restore)
# pg_restore example:
docker compose up -d db
docker compose exec db pg_restore -U flint -d flint /backup/flint_pre_migration.dump

# 4. Restart with the previous gateway image tag that does not include the bad migration
GATEWAY_IMAGE=fdb-gateway:v1.2.3 docker compose up -d fdb-gateway fke-server
```

---

## 5. Rollback Procedure

### 5.1 Image Tag Rollback (no schema change)

Use this path when the new image has a runtime bug but the schema is unchanged.

```bash
# 1. Identify the last known-good image tag
docker image ls fdb-gateway --format "{{.Tag}} {{.CreatedAt}}" | head

# 2. Pin the previous tag and restart
GATEWAY_IMAGE=fdb-gateway:v1.2.3 docker compose up -d fdb-gateway
KILN_IMAGE=fke-server:v1.2.3 docker compose up -d fke-server

# 3. Verify health
curl -sf http://localhost:8080/healthz | jq .
curl -sf http://localhost:8090/healthz | jq .
```

### 5.2 Database Snapshot Restore

Use this path when a migration must be undone or data corruption is detected.

```bash
# Pre-requisite: you have a snapshot at $SNAPSHOT_PATH (pg_dump or volume backup)

# 1. Stop all services that hold connections
docker compose down fdb-gateway fke-server

# 2. Restore snapshot (pg_restore method)
docker compose up -d db
sleep 5
docker compose exec -T db pg_restore \
  -U flint -d flint --clean --if-exists \
  < $SNAPSHOT_PATH

# 3. Roll back to the gateway image from before the migration
GATEWAY_IMAGE=fdb-gateway:<previous-tag> docker compose up -d fdb-gateway fke-server

# 4. Verify
curl -sf http://localhost:8080/healthz | jq .
```

### 5.3 Blue/Green Switchover Checklist

| Step | Action | Verification |
|---|---|---|
| 1 | Deploy new stack to green environment | `curl green:8080/healthz` returns 200 |
| 2 | Run smoke tests against green | All critical paths pass |
| 3 | Shift 10% of traffic to green | Monitor error rate in logs |
| 4 | Shift 100% of traffic to green | Monitor for 5 minutes |
| 5 | Keep blue running for 15 minutes | Ready for instant rollback |
| 6 | If errors spike: revert traffic to blue | `curl blue:8080/healthz` still 200 |
| 7 | If stable: decommission blue | `docker compose down` on blue |

---

## 6. On-Call Severity Matrix

| Severity | Description | Response SLA | Escalation |
|---|---|---|---|
| **P0** | Total outage — all `/healthz` failing or unreachable; no requests served | 15 min | Engineering lead + CTO |
| **P1** | Partial outage — one subsystem down (Kiln functions 500, GraphQL returning errors, AG-UI SSE broken) | 1 hr | On-call engineer |
| **P2** | Degraded — elevated error rate (>1% 5xx), slow response (P99 >2s), Cedar deny spike | 4 hr | Business hours engineer |
| **P3** | Minor — non-critical feature broken (HTMX renderer, OpenAPI doc stale), workaround available | Next sprint | Create ticket |

### 6.1 P0 — Immediate Response Steps

1. Page engineering lead via PagerDuty.
2. Run: `curl http://localhost:8080/healthz && curl http://localhost:8090/healthz`
3. If both fail: check `docker compose ps` — is any service in `Restarting` or `Exited`?
4. Check `docker compose logs --tail=50 fdb-gateway fke-server db`
5. Identify last deployment: `docker image inspect fdb-gateway:latest | jq '.[0].Created'`
6. If recent deploy: trigger image rollback (§5.1).

### 6.2 P1 — Subsystem Failure Steps

1. Identify which subsystem:
   - Kiln: `curl http://localhost:8090/healthz`
   - GraphQL: `curl -X POST http://localhost:8080/graphql -d '{"query":"{__typename}"}'`
   - AG-UI: check `/agents/v1/*` 500 in logs
2. Isolate the error in logs: `docker compose logs fdb-gateway 2>&1 | grep ERROR`
3. Apply targeted fix or restart the affected service.

---

## 7. Security Contacts

### 7.1 Security Team

| Role | Name | Contact |
|---|---|---|
| Security Lead | **[SECURITY_LEAD]** | security@example.com |
| On-call SecOps | **[SECOPS_ONCALL]** | +1-555-SECURITY |
| Engineering Lead | **[ENG_LEAD]** | eng-lead@example.com |
| CTO | **[CTO]** | cto@example.com |

> Replace placeholder names above with real contacts before production deployment.

### 7.2 Breach Notification Procedure

1. **Contain immediately:** If an active breach is suspected, isolate the affected
   service immediately:
   ```bash
   docker compose stop fdb-gateway fke-server
   ```
2. **Do not wipe logs:** Preserve container logs before stopping:
   ```bash
   docker compose logs fdb-gateway > /tmp/fdb-gateway-breach-$(date +%s).log
   docker compose logs fke-server  > /tmp/fke-server-breach-$(date +%s).log
   ```
3. **Alert security lead within 15 minutes** of detection.
4. **Preserve forensic state:** Do not restart or rebuild containers until the security
   lead authorizes it. Take a volume snapshot for forensics.
5. **Rotate all secrets** before restoring service:
   - `JWT_SECRET` — rotate immediately; all existing tokens are invalidated.
   - `DATABASE_URL` password — coordinate with DBA.
   - Keto + FRF service credentials.
6. **Post-incident report** due within 24 hours to security lead. Include: timeline,
   affected data, root cause (if known), remediation steps taken.

---

## 8. Monitoring Checklist

Run these checks **first** upon receiving any alert, before digging into application logs.

### 8.1 Liveness (30 seconds)

```bash
# Are both services up?
curl -sf http://localhost:8080/healthz | jq .status
curl -sf http://localhost:8090/healthz | jq .status

# Is Postgres accepting connections?
docker compose exec db pg_isready -U flint -d flint
```

### 8.2 Recent Errors (2 minutes)

```bash
# Gateway errors (last 5 minutes)
docker compose logs --since=5m fdb-gateway 2>&1 | grep -E "ERROR|WARN|panicked"

# Kiln errors (last 5 minutes)
docker compose logs --since=5m fke-server 2>&1 | grep -E "ERROR|WARN|panicked"

# Database errors
docker compose exec db psql -U flint -d flint \
  -c "SELECT pid, state, wait_event_type, wait_event, query_start, left(query,80) \
      FROM pg_stat_activity WHERE state != 'idle' ORDER BY query_start;"
```

### 8.3 Schema Health (1 minute)

```bash
# Confirm schema version incremented after any DDL
curl -sf http://localhost:8080/healthz | jq .schema_version

# Confirm latest migrations are applied
docker compose exec db psql -U flint -d flint \
  -c "SELECT version, description, installed_on \
      FROM _sqlx_migrations ORDER BY installed_on DESC LIMIT 3;"
```

### 8.4 Kiln Registry Health (1 minute)

```bash
# Active functions registered
docker compose exec db psql -U flint -d flint \
  -c "SELECT name, version, active FROM flint_kiln.functions WHERE active = true;"

# Outstanding webhook outbox entries (unprocessed BGW work)
docker compose exec db psql -U flint -d flint \
  -c "SELECT target_type, count(*) FROM flint.webhook_outbox \
      WHERE processed_at IS NULL GROUP BY target_type;"
```

### 8.5 Cedar Policy State (30 seconds)

```bash
# Are policies loaded and enabled?
docker compose exec db psql -U flint -d flint \
  -c "SELECT count(*) AS kiln_policies FROM flint_kiln.cedar_policies WHERE enabled = true;"
docker compose exec db psql -U flint -d flint \
  -c "SELECT count(*) AS meta_policies FROM flint_meta.cedar_policies WHERE enabled = true;"
```

> **Warning:** A count of 0 enabled policies means the Cedar engine is in `deny-all` mode.
> All authenticated requests will be rejected until at least one `permit` rule is inserted.

### 8.6 Connection Pool Health (1 minute)

```bash
# Current connections by application and state
docker compose exec db psql -U flint -d flint \
  -c "SELECT application_name, state, count(*) \
      FROM pg_stat_activity GROUP BY application_name, state ORDER BY count DESC;"

# Max connections vs current usage
docker compose exec db psql -U flint -d flint \
  -c "SELECT setting AS max_conn FROM pg_settings WHERE name = 'max_connections';
      SELECT count(*) AS active_conn FROM pg_stat_activity;"
```

---

*End of runbook. Review and update after every incident or major deployment.*

---

## §9 — Staging Deploy (p9-c007)

### 9.1 GitHub Actions secrets required

The `.github/workflows/deploy.yml` workflow reads the following **Environment secrets** —
add them under **Settings → Environments → staging → Environment secrets** (not repository
secrets: `deploy.yml`'s job sets `environment: ${{ inputs.environment }}`, so `staging` and
`production` — see §13 — each define their own copies of the same secret names).

> **p16-c008 migration note:** these secrets were previously named with a `STAGING_` prefix
> (`STAGING_SSH_HOST`, etc.) as plain repository secrets. `deploy.yml` now reads the generic
> names below, scoped per-Environment, so a `production` deploy target can reuse the same
> workflow without ever-growing prefixes. **Before the next staging deploy runs**, rename (or
> recreate) these secrets under the `staging` Environment using the names below — the old
> `STAGING_*`-prefixed repository secrets are no longer read by the workflow.

| Secret name | Description | Example value |
|---|---|---|
| `SSH_HOST` | Hostname or IP of the target server | `staging.example.com` |
| `SSH_USER` | SSH username on the target server | `deploy` |
| `SSH_KEY` | Contents of the **private** SSH key (`id_ed25519`) whose public key is in the server's `~/.ssh/authorized_keys` | `-----BEGIN OPENSSH PRIVATE KEY-----…` |
| `JWT_SECRET` | The raw HS256 signing key (content of `secrets/jwt_secret.txt` on the target host). Used by `mint_smoke_token.sh` to generate fresh 1-hour JWTs before each smoke test run. | *(run `rotate_secrets.sh` on the host, then copy `secrets/jwt_secret.txt` content)* |

> **Security note:** `SSH_KEY` must be a **dedicated deploy key** — never reuse a
> personal key. Rotate it quarterly or immediately after any team member departure. Use a
> **different** deploy key per environment (staging's key must not also work on production).

Separately, `STAGING_BASE_URL` (a **repository** secret, not Environment-scoped — used by
`.github/workflows/ci.yml`'s `performance` job, not `deploy.yml`) is unaffected by this
change and keeps its existing `STAGING_`-prefixed name.

### 9.2 Triggering a deploy

1. Push (or merge) a branch to `main` and confirm CI is green.
2. Navigate to **Actions → Deploy → Run workflow**.
3. Select environment `staging` and optionally override the image `tag` (default: `latest`).
4. The workflow will: pull images → `docker compose up -d` → wait for health → run smoke tests.
5. If smoke tests fail, the workflow exits non-zero. Investigate with:
   ```bash
   ssh $STAGING_SSH_USER@$STAGING_SSH_HOST "docker compose -f docker-compose.yml -f docker-compose.staging.yml logs --tail 100"
   ```

### 9.3 Manual staging deploy (fallback)

If GitHub Actions is unavailable, deploy directly from the staging host:

```bash
# 1. Pull latest images
TAG=latest \
REGISTRY=ghcr.io/<org>/flint-forge \
docker compose -f docker-compose.yml -f docker-compose.staging.yml pull

# 2. Bring the stack up
TAG=latest \
REGISTRY=ghcr.io/<org>/flint-forge \
docker compose -f docker-compose.yml -f docker-compose.staging.yml up -d --remove-orphans

# 3. Run smoke tests
BASE_URL=http://localhost:8080 \
KILN_URL=http://localhost:8090 \
SMOKE_TOKEN=<jwt> \
./smoke_test.sh
```

### 9.4 Compose file hierarchy

| File | Purpose |
|---|---|
| `docker-compose.yml` | Base definition (build context, env, ports, healthchecks) |
| `docker-compose.staging.yml` | Staging overrides: image refs, restart policies, CPU/memory limits |
| `docker-compose.prod.yml` | Production hardening: no exposed ports, TLS termination via reverse proxy |

Always layer files in this order: `docker-compose.yml` first, then the environment overlay.

---

## §10 — TLS Termination (p10-c001)

### 10.1 How TLS works in the production stack

Caddy (`caddy:2-alpine`) acts as the sole internet-facing service. It terminates
TLS and reverse-proxies to the internal services over Docker's bridge network:

```
Internet → :443 HTTPS → Caddy → fdb-gateway:8080 (internal)
Internet → :80  HTTP  → Caddy redirects to HTTPS automatically
```

`fdb-gateway` and `fke-server` do **not** expose ports 8080/8090 in the prod
overlay (`ports: !reset []`). They are only reachable from Caddy on the internal
network.

### 10.2 Required environment variables

Set these before starting the prod stack (`docker-compose.prod.yml` will
refuse to start without them):

| Variable | Example | Description |
|---|---|---|
| `FLINT_DOMAIN` | `forge.example.com` | Public domain; Caddy requests a cert for this name |
| `CADDY_TLS_EMAIL` | `ops@example.com` | ACME account email for Let's Encrypt notifications |

Add them to your `.env` file on the host (or inject via the secrets rotation
script — see §10.5):

```bash
FLINT_DOMAIN=forge.example.com
CADDY_TLS_EMAIL=ops@example.com
```

### 10.3 First-run TLS provisioning

On first `docker compose ... up -d`, Caddy contacts the Let's Encrypt ACME
server and requests a certificate:

1. Let's Encrypt sends an HTTP-01 challenge to `http://{FLINT_DOMAIN}/.well-known/acme-challenge/…`
2. Caddy answers the challenge automatically (port 80 must be reachable from the internet)
3. Certificate is issued and stored in the `caddy_data` Docker volume
4. Caddy begins serving HTTPS

**DNS prerequisite:** `FLINT_DOMAIN` must resolve to the server's public IP
before running the stack. Certificate provisioning will fail (and retry
with exponential backoff) if DNS is not yet propagated.

**Verify provisioning:**

```bash
# Wait for Caddy to log "certificate obtained"
docker compose -f docker-compose.yml -f docker-compose.prod.yml logs caddy | grep -i "certificate\|obtained\|error"

# Check the cert from outside
curl -I https://${FLINT_DOMAIN}/healthz
# Expect: HTTP/2 200
```

### 10.4 Certificate renewal

Caddy renews certificates automatically **30 days before expiry** (Let's
Encrypt certs are valid for 90 days). No operator action is required unless
the `caddy_data` volume is deleted.

**Verify next renewal date:**

```bash
docker compose -f docker-compose.yml -f docker-compose.prod.yml exec caddy \
  caddy certificates
```

### 10.5 Troubleshooting TLS

| Symptom | Likely cause | Fix |
|---|---|---|
| `ERR_SSL_PROTOCOL_ERROR` in browser | Cert not yet provisioned | Check Caddy logs; verify DNS; wait up to 2 minutes |
| `Caddy exited with code 1` on startup | `FLINT_DOMAIN` or `CADDY_TLS_EMAIL` not set | Confirm both are in `.env` |
| ACME rate limit error | More than 5 cert requests per domain per hour | Wait 1 hour; Caddy retries automatically |
| Port 80 unreachable | Firewall blocking HTTP challenge | Open port 80 on the host firewall |
| Let's Encrypt staging / `tls internal` | Self-signed cert, browser warning | For local dev use `tls internal` in Caddyfile (see §10.6) |
| Certificate expired | `caddy_data` volume deleted | Delete old cert, restart Caddy: see §10.5.1 |

#### §10.5.1 — Force certificate re-issuance

```bash
# Stop the stack
docker compose -f docker-compose.yml -f docker-compose.prod.yml down

# Remove the certificate volume (destructive — cert will be re-requested)
docker volume rm flint-forge_caddy_data

# Restart — Caddy will re-provision
docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d
```

### 10.6 Local / self-signed TLS (no ACME)

For staging without a public domain, replace `tls {$CADDY_TLS_EMAIL}` with
`tls internal` in `docker/caddy/Caddyfile`:

```
{$FLINT_DOMAIN} {
    reverse_proxy fdb-gateway:8080
    tls internal
}
```

Caddy generates a locally-trusted CA and certificate. Add the Caddy root CA to
your browser/OS trust store to avoid certificate warnings:

```bash
docker compose ... exec caddy caddy trust
```

### 10.7 Secrets rotation (p10-c002)

All production secrets are managed as Docker Compose secret files under `secrets/`
(gitignored). Use `scripts/rotate_secrets.sh` to generate or rotate them.

#### 10.7.1 Initial secret generation (first deploy)

```bash
# Generate all secret files and update .env
CADDY_TLS_EMAIL=ops@example.com ./scripts/rotate_secrets.sh
```

This creates:
- `secrets/jwt_secret.txt` — JWT signing key (fdb-gateway)
- `secrets/postgres_password.txt` — PostgreSQL password (db + app containers)
- `secrets/caddy_tls_email.txt` — ACME email (caddy)

And updates `.env` with `DATABASE_URL` using the new password.

#### 10.7.2 Quarterly rotation

Rotate secrets every 90 days (or immediately after any suspected exposure):

```bash
# Stop the stack
docker compose -f docker-compose.yml -f docker-compose.prod.yml down

# Rotate — generates new random values for jwt_secret and postgres_password
CADDY_TLS_EMAIL=ops@example.com ./scripts/rotate_secrets.sh

# Restart with new secrets
docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d

# Verify health
BASE_URL=https://forge.example.com SMOKE_TOKEN=<new-jwt> ./scripts/smoke_test.sh
```

> **Note:** Rotating `postgres_password` changes the PostgreSQL superuser
> password. The `db` container reads it via `POSTGRES_PASSWORD_FILE` on startup,
> and `.env` is updated automatically. Existing database connections from app
> containers will reset on restart.

#### 10.7.3 Secret file paths inside containers

| Secret name | Mount path | Consumer |
|---|---|---|
| `jwt_secret` | `/run/secrets/jwt_secret` | `fdb-gateway` (`FLINT_JWT_SECRET_FILE`) |
| `postgres_password` | `/run/secrets/postgres_password` | `db` (`POSTGRES_PASSWORD_FILE`) |
| `caddy_tls_email` | `/run/secrets/caddy_tls_email` | `caddy` |

---

## §11 — Staging Token Rotation (p11-c006)

### 11.1 Overview

`scripts/mint_smoke_token.sh` mints a self-signed HS256 JWT with a **1-hour
expiry** (`exp = now + 3600`). It replaces the long-lived static
`STAGING_SMOKE_TOKEN` secret previously stored in GitHub Actions, which had an
unlimited lifetime and represented a standing credential risk.

The generated token carries:

| Claim | Value |
|---|---|
| `sub` | `smoke` |
| `role` | `authenticated` |
| `exp` | Unix timestamp — `now + 3600` (1 hour) |
| `iat` | Unix timestamp — `now` |

### 11.2 Manual use

```bash
# Use JWT_SECRET from the environment
JWT_SECRET=<your-key> ./scripts/mint_smoke_token.sh

# Use the secret file (after running rotate_secrets.sh)
./scripts/mint_smoke_token.sh        # reads secrets/jwt_secret.txt automatically

# Capture for use in another command
TOKEN=$(JWT_SECRET=<your-key> ./scripts/mint_smoke_token.sh)
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/graphql
```

Decode the payload without a library:

```bash
JWT_SECRET=<your-key> ./scripts/mint_smoke_token.sh | \
  cut -d. -f2 | \
  awk '{n=length($0)%4; if(n>0) for(i=n;i<4;i++) printf "="; print}' | \
  base64 -d 2>/dev/null | python3 -m json.tool
```

### 11.3 Integration with deploy.yml

The `deploy.yml` workflow mints a fresh token immediately before running smoke
tests. It reads the key from the `STAGING_JWT_SECRET` repository/environment
secret (see §9.1) and injects the minted token into `$GITHUB_ENV` so that the
subsequent SSH command can pass it to `smoke_test.sh`.

```yaml
- name: Mint smoke token
  run: |
    chmod +x scripts/mint_smoke_token.sh
    SMOKE_TOKEN=$(JWT_SECRET="${{ secrets.STAGING_JWT_SECRET }}" \
      ./scripts/mint_smoke_token.sh)
    echo "SMOKE_TOKEN=${SMOKE_TOKEN}" >> "$GITHUB_ENV"

- name: Run smoke tests
  run: |
    ssh ... "SMOKE_TOKEN='${SMOKE_TOKEN}' ./smoke_test.sh"
```

### 11.4 Why this is more secure than a static STAGING_SMOKE_TOKEN

| Property | Static `STAGING_SMOKE_TOKEN` | Dynamic `mint_smoke_token.sh` |
|---|---|---|
| Token lifetime | Unlimited (or manually rotated) | 1 hour maximum |
| Blast radius if leaked | Token valid until manually revoked | Token expires within 1 hour |
| GitHub secret value | Full JWT — usable immediately by anyone who reads it | Raw signing key — requires running the script to produce a usable token |
| Rotation | Manual: regenerate token + update secret | Key rotation via `rotate_secrets.sh`; all old tokens expire naturally |
| CI coupling | Tight — secret IS the credential | Loose — secret is a key; the credential is derived per-run |

Tokens expire after 1 hour by design. A leaked CI log or artifact that contains
the minted `SMOKE_TOKEN` poses only a brief, time-bounded risk compared with a
static bearer token that remains valid indefinitely.

---

## §12 — Staging JWT Secret Rotation (p14-c004)

### 12.1 Overview

`scripts/rotate_staging_jwt.sh` rotates the raw HS256 signing key stored in the
`staging` GitHub Environment's `JWT_SECRET` secret (p16-c008: renamed from the
repo-level `STAGING_JWT_SECRET` — see §9.1/§13). It also writes the same key
locally to `secrets/jwt_secret.txt` so operators can mint smoke tokens on the
staging host during troubleshooting.

The script is intended to be run manually when:

- The key is suspected to be compromised.
- A team member with access to the secret leaves the project.
- The quarterly rotation cycle comes due.
- You want to invalidate all previously minted staging smoke tokens.

### 12.2 Prerequisites

| Requirement | Verification |
|---|---|
| `gh` CLI installed | `gh --version` |
| Authenticated to GitHub | `gh auth status` |
| Push access to the repository | Confirm you can open the repo settings |
| Local `secrets/` directory writable | `mkdir -p secrets` |

### 12.3 Running the rotation

```bash
# Rotate for real
./scripts/rotate_staging_jwt.sh

# Preview only — no files or secrets are changed
./scripts/rotate_staging_jwt.sh --dry-run
```

The script performs the following steps:

1. Generates a new 32-byte hex random string with `openssl rand -hex 32`.
2. Writes it to `secrets/jwt_secret.txt` with `chmod 600`.
3. Updates the `staging` Environment's `JWT_SECRET` via `gh secret set JWT_SECRET --env staging`.

### 12.4 Applying the new key

GitHub Actions uses the updated secret immediately, but `fdb-gateway` reads the
key from the Docker secret file on startup. Restart the staging stack to load it:

```bash
docker compose -f docker-compose.yml -f docker-compose.staging.yml up -d
```

Then mint a fresh token and validate the deployment:

```bash
TOKEN=$(./scripts/mint_smoke_token.sh)
BASE_URL=https://forge.example.com KILN_URL=http://localhost:8090 \
  SMOKE_TOKEN=$TOKEN ./scripts/smoke_test.sh
```

### 12.5 Rollback

If the rotation breaks staging, restore the previous value of the `staging`
Environment's `JWT_SECRET` from a secure backup and re-run the stack restart.
Because old tokens are signed with the previous key, they will validate again
once the gateway is using that key.

### 12.6 Security notes

- Never commit `secrets/jwt_secret.txt`; the directory is already gitignored.
- Do not print the secret value in CI logs. The script passes it directly to `gh`
  and redacts it from output.
- Treat the `staging` Environment's `JWT_SECRET` with the same access controls
  as production signing keys; staging keys can mint tokens that exercise the
  same code paths.

---

## §13 — Production Deploy Setup (p16-c008)

### 13.1 Overview

`.github/workflows/deploy.yml` supports both `staging` and `production` as the
`environment` input. The workflow itself is identical for both — what differs
is which GitHub Environment's secrets it reads (`SSH_HOST`/`SSH_USER`/
`SSH_KEY`/`JWT_SECRET`, per §9.1) and which compose overlay it layers on top of
the base `docker-compose.yml` (`docker-compose.staging.yml` vs
`docker-compose.prod.yml`).

**This setup requires a human operator with GitHub repo admin access — it
cannot be completed by an agent.** The steps below are the one-time setup;
after that, triggering a deploy is the same `Actions → Deploy → Run workflow`
flow already used for staging (§9.2), just with `production` selected.

### 13.2 One-time setup (operator, GitHub repo admin)

1. **Create the `production` GitHub Environment.** Repo → **Settings →
   Environments → New environment** → name it exactly `production`.
2. **Add required reviewers.** On the `production` environment's configuration
   page, enable **Required reviewers** and add at least one person (not the
   person triggering the deploy, if your policy requires separation of
   duties). This is what makes `production` deploys pause for human approval
   — `staging` should NOT have this restriction, so routine staging deploys
   stay fast.
3. **(Optional) Restrict deployment branches** to `main` only, so a
   production deploy can't accidentally run from a feature branch.
4. **Add the `production` Environment's secrets** (Settings → Environments →
   production → Environment secrets) — see §9.1 for the secret names
   (`SSH_HOST`, `SSH_USER`, `SSH_KEY`, `JWT_SECRET`). These are **separate
   values from staging's** — provision a dedicated production SSH deploy key
   and a dedicated production JWT signing secret. Do not reuse staging's.
5. **If `staging`'s secrets are still under the old `STAGING_`-prefixed
   repository-secret names**, migrate them to the `staging` Environment under
   the new generic names now too (§9.1) — both environments should follow the
   same pattern.
6. **Provision the production host's Docker secrets** (separate from the
   GitHub Actions secrets above): run `scripts/rotate_secrets.sh` directly on
   the production host to generate `secrets/jwt_secret.txt`,
   `secrets/postgres_password.txt`, and `secrets/caddy_tls_email.txt` — these
   are what `docker-compose.prod.yml`'s `secrets:` block mounts into the
   containers (§10.7). The `JWT_SECRET` GitHub Actions secret (step 4) and the
   `secrets/jwt_secret.txt` file on the host should hold the **same** value,
   since `mint_smoke_token.sh`'s tokens (signed with the GitHub secret) must
   validate against `fdb-gateway`'s `FLINT_JWT_SECRET` (loaded from the host
   file via the entrypoint script, §11).
7. **Set `FLINT_DOMAIN`/`CADDY_TLS_EMAIL`** on the production host's `.env`
   (§10.2) — required for `docker-compose.prod.yml`'s Caddy TLS termination.

### 13.3 First production deploy (operator)

Do not trigger the first production deploy the same way as a routine one.
Recommended sequence:

1. Confirm the host is reachable and has Docker + the pinned compose files
   present (mirroring §9.3's manual-fallback steps, but for the production
   host).
2. Trigger `Actions → Deploy → Run workflow` with `environment: production`.
3. The job will pause at the "Deploy to production" step awaiting the
   required reviewer's approval (§13.2 step 2) — approve it from the Actions
   run page.
4. Watch the run through to the smoke-test step. If smoke tests fail, the
   workflow exits non-zero and the stack is left in whatever partial state
   `docker compose up -d` produced — do not treat a failed first deploy as
   safe to ignore; investigate via SSH before retrying.
5. Record the outcome (date, commit SHA, reviewer, pass/fail) somewhere your
   team tracks operational changes — this repo does not currently have an
   automated deploy-history log beyond the GitHub Actions run history itself.

### 13.4 Backups — automated `wal-g` PITR (see also §5 — Rollback Procedure)

**Automation is implemented; the restore drill still has not been executed.**
`docker-compose.prod.yml`'s Postgres container still carries the "prefer a
managed database" caveat at its top, and that caveat stays until an operator
completes 13.4.3 below — a backup that has never been restored is not a
backup, only a hope.

#### 13.4.1 Architecture

Two independent, complementary mechanisms cover PITR:

- **Continuous WAL archiving** — the `db` service's `command:` sets
  `archive_mode=on` and `archive_command='wal-g wal-push %p'`
  (`archive_timeout=60` bounds how long a quiet database can go between
  archived segments). `wal-g` is baked into the production image
  (`images/postgres18/Dockerfile`) at a pinned version with a checked SHA256.
  `images/postgres18/entrypoint-walg.sh` (the image's `ENTRYPOINT`) translates
  the `walg_s3_access_key`/`walg_s3_secret_key` Docker secrets into
  `AWS_ACCESS_KEY_ID`/`AWS_SECRET_ACCESS_KEY` before starting Postgres, so
  `archive_command` has credentials available when it fires.
- **Periodic full base backups** — the `backup` service
  (`docker-compose.prod.yml`) runs `images/postgres18/backup-loop.sh` in a
  loop (`WALG_BACKUP_INTERVAL_SECS`, default daily), calling
  `wal-g backup-push` against a read-only mount of the same `postgres_data`
  volume while driving `pg_backup_start()`/`pg_backup_stop()` over a network
  connection to `db`.

Both mechanisms **no-op cleanly** (log a message, do nothing destructive) when
the `walg_s3_access_key`/`walg_s3_secret_key` secret files aren't present —
local dev, CI, and staging are unaffected by default.

#### 13.4.2 Operator setup

1. Provision an S3-compatible bucket (AWS S3, or any S3-API-compatible
   provider) dedicated to this backup target.
2. Write the access key and secret key to `secrets/walg_s3_access_key.txt` and
   `secrets/walg_s3_secret_key.txt` on the production host (`chmod 600`,
   matching the convention in `scripts/rotate_secrets.sh` — these two files
   are **not** generated by that script since the values come from the
   storage provider, not a local random generator).
3. Set `WALG_S3_PREFIX` (e.g. `s3://flint-forge-backups/prod`) and, if your
   provider requires it, `WALG_AWS_REGION` in the production host's `.env`.
4. Restart the stack so `db` and `backup` pick up the new secrets/env:
   `docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d db backup`
5. Confirm archiving is live:
   `docker compose ... logs backup` should show `backup-loop: starting,
   interval=...`, and `SELECT * FROM pg_stat_archiver;` on `db` should show
   `archived_count` increasing and `last_failure` not advancing.

#### 13.4.3 Restore drill (operator — required before the "staging only" caveat can be removed)

Run `./scripts/restore_pg_pitr.sh --latest` (or `--target-time '<UTC timestamp>'`
for a specific point in time) against a **non-production** copy of the stack
first, then again against production during a planned maintenance window. The
script stops `db`, fetches the base backup + replays WAL via `wal-g`, and
prints verification steps. Record here once executed:

| Date | Target | Verified by | Result |
|---|---|---|---|
| _(not yet run)_ | | | |

Until a row exists in that table with a passing result, treat automated
backups as **unverified** — the mechanism running is not the same claim as
the mechanism working.

### 13.5 LLM background worker default

`llm.enable_background_worker` (a Postgres `shared_preload_libraries` +
postmaster-context GUC in `ext-flint-llm`) defaults to **disabled** — LLM
calls run synchronously. This was a deliberate decision, not an oversight:
enabling it means every deployment runs an additional persistent background
worker process, requires `shared_preload_libraries` to be configured
correctly at postmaster start (a restart to toggle), and adds an operational
surface (the worker's own health/monitoring) that isn't justified until there
is an actual need for async LLM job processing. Operators who need async
processing (e.g., long-running embedding batch jobs that shouldn't block a
request) should explicitly opt in by setting `llm.enable_background_worker =
on` in their Postgres configuration and ensuring the extension is listed in
`shared_preload_libraries`.

