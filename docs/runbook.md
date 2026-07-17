# Flint Forge — Operations Runbook

> **Audience:** On-call engineers, SREs, and DevOps operators.
> **Last updated:** 2026-07-13
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

# REQUIRED — bearer-token verification (forge-identity::verify_and_build,
# called by fdb-auth::rls_from_bearer on every authenticated request). The
# gateway fetches flint-gate's JWKS once per process lifetime and verifies
# inbound JWTs against it. Only RS256/RS384/RS512/ES256/ES384 are accepted.
# Both vars are hard-required: if either is unset, EVERY authenticated
# request fails with 401 (there is no fallback verification path).
FLINT_GATE_JWKS_URL=https://gate.example.com/.well-known/jwks.json
FLINT_GATE_ISSUER=https://gate.example.com

# Optional — `aud` claim validation is skipped if unset
FLINT_GATE_AUDIENCE=

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

> **`FLINT_JWT_SECRET` / `JWT_SECRET` is a separate, unrelated variable —
> `fdb-gateway` never reads it.** It is consumed only by `forge-cli token mint`
> (a local HS256 token-minting helper) and by the Docker Compose / entrypoint
> secret plumbing described in §10.7.3, which sets the env var in the gateway
> container even though nothing there consumes it. Tokens produced by
> `forge token mint` or `scripts/mint_smoke_token.sh` are signed HS256 and
> **will not authenticate** against this gateway — see Error 3 and §11/§12
> below for the full implication.

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
# Check that the JWKS env vars are set in the gateway container — both are
# hard-required by forge-identity::verify_and_build. If either is missing,
# EVERY request fails closed with the generic "invalid or expired token" 401,
# regardless of the token presented.
docker compose exec fdb-gateway env | grep -i flint_gate

# Decode the token (without verification) to inspect claims — confirm `alg`
# in the header and `iss`/`aud` in the payload.
echo "<base64.payload.sig>" | cut -d. -f2 | base64 -d | jq .

# Check gateway logs for the underlying verification error (logged at WARN,
# never returned to the client — the response body is always the generic
# message above regardless of cause)
docker compose logs fdb-gateway 2>&1 | grep "bearer verification failed"
```

**Remediation**

1. **`FLINT_GATE_JWKS_URL` or `FLINT_GATE_ISSUER` unset:** logs show
   `required environment variable not set: FLINT_GATE_JWKS_URL` (or `..._ISSUER`).
   Set both in `.env`, then restart the gateway:
   ```bash
   docker compose down fdb-gateway
   docker compose up -d fdb-gateway
   ```
2. **Unsupported algorithm:** logs show `verification failed: unsupported algorithm: ...`.
   `forge-identity::verify_and_build` only accepts RS256/RS384/RS512/ES256/ES384 —
   **HS256 tokens are always rejected**, including anything minted by
   `forge-cli token mint` or `scripts/mint_smoke_token.sh` (see §2.2). The
   token must be re-issued by flint-gate (or a JWKS-compatible test issuer)
   using an asymmetric algorithm.
3. **Unknown `kid` / JWKS fetch or parse failure:** logs show `unknown \`kid\`: ...`,
   `failed to fetch JWKS: ...`, or `failed to parse JWKS: ...`. Confirm
   `FLINT_GATE_JWKS_URL` is reachable from inside the container and serves a
   valid JWK Set containing the key that signed the token. Note: the JWKS is
   cached for the lifetime of the process (`forge-identity::jwks`) — a gateway
   restart is required to pick up a rotated flint-gate signing key.
4. **Wrong issuer or audience:** confirm the token's `iss` claim matches
   `FLINT_GATE_ISSUER` exactly, and (if `FLINT_GATE_AUDIENCE` is set) that
   `aud` matches too.
5. **Expired token or clock skew:** ensure the server clock is synchronized
   (NTP). Token `exp` validation fails if the gateway clock is ahead of the
   token's `iat`/`exp`.

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

> **`v1.0.0` image location note:** the `docker.yml` tag-triggered CI run for `v1.0.0`
> failed (`repository name must be lowercase`, fixed in a later commit not part of
> that tag), so no `ghcr.io/know-me-tools/*:v1.0.0` images exist. `v1.0.0` images
> were instead published manually to `docker.io/tribehealth/flint-gateway:v1.0.0`
> and `docker.io/tribehealth/flint-kiln:v1.0.0`, built from the exact `v1.0.0` source
> with the Rust base image locally bumped (`1.85-slim` → `1.96-slim`) to work around
> an unrelated `cargo-chef` toolchain incompatibility. See the
> [`v1.0.0` release notes](https://github.com/Know-Me-Tools/flint-forge/releases/tag/v1.0.0)
> for details. `v1.0.1`+ images are published to `ghcr.io/know-me-tools/*` by CI as normal.

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
   - **flint-gate signing key** — rotate immediately at the source (flint-gate,
     out of this repo) and confirm its JWKS document drops the old `kid`; this
     is what actually invalidates existing tokens, since `fdb-gateway` verifies
     exclusively against that JWKS (see §2.2). Rotating `FLINT_JWT_SECRET` /
     `secrets/jwt_secret.txt` does **not** invalidate any token the gateway
     accepts — that variable is not part of the verification path.
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
| `JWT_SECRET` | The raw HS256 signing key (content of `secrets/jwt_secret.txt` on the target host). Used by `mint_smoke_token.sh` to generate fresh 1-hour JWTs before each smoke test run. **⚠ See warning below — the resulting token is currently rejected by the gateway.** | *(run `rotate_secrets.sh` on the host, then copy `secrets/jwt_secret.txt` content)* |
| `STAGING_BASE_URL` | Public HTTPS base URL of the staging stack — used by the k6 performance regression job | `https://forge.example.com` |

> **Security note:** `SSH_KEY` must be a **dedicated deploy key** — never reuse a
> personal key. Rotate it quarterly or immediately after any team member departure. Use a
> **different** deploy key per environment (staging's key must not also work on production).

Separately, `STAGING_BASE_URL` (a **repository** secret, not Environment-scoped — used by
`.github/workflows/ci.yml`'s `performance` job, not `deploy.yml`) is unaffected by this
change and keeps its existing `STAGING_`-prefixed name.

> **⚠ Known issue — smoke-test auth is currently broken.** `fdb-gateway`'s
> bearer verification (`forge-identity::verify_and_build`, §2.2) only accepts
> JWKS-verified RS256/RS384/RS512/ES256/ES384 tokens and requires
> `FLINT_GATE_JWKS_URL`/`FLINT_GATE_ISSUER` to be set wherever the gateway
> runs. Neither the staging compose overlay nor this workflow sets those, and
> `mint_smoke_token.sh` mints an HS256 token, which the gateway rejects
> outright. As configured, the "Mint smoke token" + "Run smoke tests" steps
> above will not successfully authenticate against a gateway that has JWKS
> verification enabled. Fixing this requires either a JWKS-compatible token
> issuer reachable from staging, or an explicit, intentionally-scoped test
> auth path — that is tracked as follow-up work, not something this runbook
> update resolves. Until then, treat smoke-test 401s as expected rather than
> a regression signal.

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
| `jwt_secret` | `/run/secrets/jwt_secret` | `docker/fdb-gateway/entrypoint.sh` reads this file and exports it as `FLINT_JWT_SECRET` — but no code in `fdb-gateway` reads that env var. It is only useful if you separately run `forge-cli token mint` inside the same container. |
| `postgres_password` | `/run/secrets/postgres_password` | `db` (`POSTGRES_PASSWORD_FILE`) |
| `caddy_tls_email` | `/run/secrets/caddy_tls_email` | `caddy` |

> Real inbound-auth verification is driven entirely by `FLINT_GATE_JWKS_URL` /
> `FLINT_GATE_ISSUER` / `FLINT_GATE_AUDIENCE` (plain env vars, not Docker
> secrets today — see §2.2), not by the `jwt_secret` Docker secret above.

---

## §11 — Staging Token Rotation (p11-c006)

> **⚠ Status: the token minted by this mechanism does not authenticate against
> the current gateway.** This section was written when `fdb-auth` verified
> self-signed HS256 tokens against a shared secret. `fdb-gateway` now verifies
> exclusively via JWKS (`forge-identity::verify_and_build`, asymmetric
> algorithms only — see §2.2 and Error 3). Everything below accurately
> describes what `mint_smoke_token.sh` and `deploy.yml` currently *do*; it no
> longer describes a working end-to-end auth flow. Treat this as a record of
> the legacy design pending a follow-up fix, not a working procedure.

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

> **⚠ Status: same caveat as §11** — this rotates the HS256 signing key used
> by `mint_smoke_token.sh`, which is unrelated to the JWKS-based verification
> `fdb-gateway` actually performs. Rotating `STAGING_JWT_SECRET` does not
> invalidate or affect any token the gateway accepts.

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

If the rotation breaks something that depends on `mint_smoke_token.sh` output,
restore the previous value of the `staging` Environment's `JWT_SECRET` from a
secure backup and re-run the stack restart. This key only affects tokens
minted by `mint_smoke_token.sh` / `forge token mint` — it has no effect on the
gateway's JWKS-based verification (§2.2), which is unaffected by this rotation
either way.

### 12.6 Security notes

- Never commit `secrets/jwt_secret.txt`; the directory is already gitignored.
- Do not print the secret value in CI logs. The script passes it directly to `gh`
  and redacts it from output.
- Treat the `staging` Environment's `JWT_SECRET` with the same access controls
  as production signing keys; staging keys can mint tokens that exercise the
  same code paths.

---

## §13 — Production Deploy Setup (p16-c008)

### 13.1 Overview — corrected architecture

**This section originally assumed a single SSH-reachable production host and
S3-backed backups. Both assumptions were wrong for this org's actual
infrastructure and have been replaced.** flint-forge's real production
target is Kubernetes: a shared, multi-tenant AKS cluster (`main`, resource
group `prometheus-rg`, subscription "Azure subscription 1") already running
ArgoCD in-cluster and used by several other projects (firecrawl, hotseaters,
matrix, stalwart, and others — see `kubectl get namespaces`). There is no
single "production host" and no S3 — the org's cloud provider is Azure.
There is also no separate "staging" environment: short of an actual client
deployment, the only environment that exists is this repo's own
`docker-compose.yml` stack, which §9's staging-deploy content (SSH to a
`STAGING_*`-secrets host) predates and no longer reflects — treat local
docker-compose as the only pre-production environment until a real client
deployment exists.

Everything below is fully automated — no operator needs to run any of it by
hand for the system to function. What follows documents *how* the automation
works and how to extend it (e.g. onboarding a new client), not a checklist
to execute.

### 13.2 Components

| Component | What it does | Where |
|---|---|---|
| AKS cluster `main` | Shared Kubernetes cluster, OIDC issuer + Workload Identity enabled | resource group `prometheus-rg`, region `centralus` |
| ArgoCD (in-cluster) | Watches this repo, reconciles each tenant's `Application` | namespace `argocd` on `main` — not a separate control-plane cluster |
| `deploy/argocd/flint-forge-applicationset.yaml` | One `Application` per tenant (list generator) | applied once: `kubectl apply -f deploy/argocd/flint-forge-applicationset.yaml` |
| `deploy/helm/flint-forge/` | The chart every tenant `Application` deploys | this repo |
| `deploy/helm/flint-forge/values-<tenant>.yaml` | Per-tenant image tags + backup config (non-secret) | this repo — CI commits new image tags here |
| ACR `prometheusagsacr.azurecr.io` | Shared container registry | resource group `prometheus-rg` |
| Azure AD app `github-actions-aks-deploy` | GitHub OIDC → Azure identity for CI (AcrPush only — no cluster access; ArgoCD does the actual deploy) | shared across projects on this cluster, appId `dcca803b-47f0-496d-b1d4-e1b0c0cfc79e` |
| Managed identity `flint-forge-walg-identity` | Workload Identity for wal-g's Azure Blob access (no static keys) | resource group `flint-forge-rg` |
| Storage account `stflintforgebakc69689`, container `pg-backups` | wal-g backup target | resource group `flint-forge-rg`, region `centralus` |

None of the values above are secrets — Client IDs, storage account names,
and resource names carry no access on their own under OIDC/Workload
Identity federation (there is no shared secret to leak), so they're
committed directly in `values-<tenant>.yaml` and this doc rather than
hidden in a secret store.

### 13.3 Deploy flow (fully automated, no SSH)

1. A push to `main` touching `crates/**`, `docker/**`, `images/postgres18/**`,
   or the Helm chart triggers `.github/workflows/deploy-aks.yml`.
2. That workflow authenticates to Azure via GitHub OIDC (no stored Azure
   credential — `azure/login@v2` with `vars.AZURE_CLIENT_ID/TENANT_ID/
   SUBSCRIPTION_ID`, federated per `github-actions-aks-deploy`'s
   `repo:Know-Me-Tools/flint-forge:...` subjects), builds and pushes the
   `gateway`, `kiln`, and `postgres18` images to ACR, then commits the new
   commit-sha tag into `deploy/helm/flint-forge/values-<tenant>.yaml`.
3. That commit **is** the deploy — ArgoCD (already watching this repo,
   `selfHeal: true`) picks it up and reconciles the cluster automatically.
   Nothing in CI ever touches `kubectl`/the cluster directly.
4. The `build-and-push` job runs under the `production` GitHub Environment
   (required reviewers) — the same human-approval gate originally set up for
   the retired SSH workflow now protects this one (a second,
   environment-scoped federated credential was added for this:
   `repo:Know-Me-Tools/flint-forge:environment:production`).

Watch a rollout: `kubectl get application <tenant> -n argocd -w` (requires
`az aks get-credentials -g prometheus-rg -n main` + cluster access — this is
an operator/debugging action, not something CI needs).

### 13.4 Backups — wal-g to Azure Blob Storage via Workload Identity

**No static storage keys exist anywhere in this system.** The postgres pod's
ServiceAccount (`<release>-postgres`) carries an `azure.workload.identity/
client-id` annotation pointing at `flint-forge-walg-identity`; the AKS
workload-identity webhook injects `AZURE_CLIENT_ID`/`AZURE_TENANT_ID`/
`AZURE_FEDERATED_TOKEN_FILE` into any pod labeled `azure.workload.identity/
use: "true"` using that ServiceAccount, and wal-g's Azure backend picks
those up via the standard Azure default-credential chain automatically. The
identity is granted `Storage Blob Data Contributor` scoped to exactly the
`pg-backups` container — nothing broader.

- **Continuous WAL archiving**: `archive_command=wal-g wal-push %p`, set
  directly on the postgres StatefulSet's container args
  (`deploy/helm/flint-forge/templates/postgres.yaml`) when
  `backup.enabled=true`.
- **Periodic base backups**: `deploy/helm/flint-forge/templates/
  backup-cronjob.yaml`, a CronJob that `kubectl exec`s `wal-g backup-push`
  directly in the postgres pod (avoids needing a `ReadWriteMany` volume for
  a second pod — `wal-g` needs local `$PGDATA` filesystem access). This
  trigger pod itself carries no Azure identity — only RBAC to `exec` into
  the postgres pod, since the actual wal-g process and its Workload Identity
  live inside that pod.
- **The docker-compose S3 path (`docker-compose.prod.yml`, `images/
  postgres18/entrypoint-walg.sh`, `backup-loop.sh`) still exists but is not
  the supported production path** — this org will never deploy that way.
  Those scripts remain functional no-ops for local/dev compose (no secret
  files present there → they exit cleanly without archiving) and are left
  in place only for anyone running a standalone single-host compose
  instance outside this org's AKS cluster.

### 13.5 Restore drill — automated, recurring, non-disruptive

`deploy/helm/flint-forge/templates/restore-drill-cronjob.yaml`
(`backup.restoreDrill.enabled`, weekly by default) fetches the latest wal-g
backup into a **throwaway `emptyDir`** — never the live postgres
`PersistentVolumeClaim` — starts Postgres against it, confirms it leaves
recovery mode, runs a verification query, and reports pass/fail via pod
logs and exit code (`kubectl logs -n <namespace> job/<latest-restore-drill-job>`,
or `kubectl get events -n <namespace> --field-selector
involvedObject.kind=Job`). Because it never touches the production volume,
this can safely run unattended on a schedule — it proves backups are
restorable continuously, not just once.

This replaces the old `scripts/restore_pg_pitr.sh` (still present and usable
for a real DR failover exercise against docker-compose — it deliberately
stops the live `db` service, which the automated drill above does not) as
the mechanism that satisfies the proposal's "prove data comes back, not just
that a backup file was written" requirement. Its first real run happens
automatically once flint-forge is deployed and at least one backup cycle has
completed — there was nothing to restore before that point, so no drill
result exists yet:

| Date | Target | Verified by | Result |
|---|---|---|---|
| _(pending — first scheduled run after initial deployment + one backup cycle)_ | | | |

Check the current status any time with:
`kubectl get cronjob <tenant>-restore-drill -n <tenant>` and
`kubectl logs -n <tenant> -l app.kubernetes.io/component=restore-drill --tail=50`.

### 13.6 Performance baselines — automated against docker-compose

`.github/workflows/ci.yml`'s `performance` job runs on every push to `main`
(and on manual dispatch): it builds and starts the `docker-compose.yml`
stack in the CI runner (the only "staging" that exists — see §13.1), runs
`perf/k6/regression.js`, and uses `regression.js`'s own `handleSummary()`
output (`perf-summary.json`) to rewrite its own P99 thresholds
(`ceil(measured_p99 * 1.20)`, rounded to the nearest 5ms),
`BASELINE_DATE`/`BASELINE_SOURCE`, and a results file under `perf/results/`
— then commits that back to `main` (`[skip ci]`). No one runs k6 by hand for
this to work; the baseline keeps itself current on every merge.

### 13.7 Onboarding a new tenant

Since there is no single production deployment, adding a client is a
repeatable, scriptable sequence, not a new pipeline:

1. Provision a federated credential for that tenant's postgres pod:
   ```bash
   az identity federated-credential create \
     --name <tenant>-postgres-sa \
     --identity-name flint-forge-walg-identity \
     --resource-group flint-forge-rg \
     --issuer "https://centralus.oic.prod-aks.azure.com/d48ebfee-d3e4-474c-8616-509f441e438f/51e63d24-bb81-414a-8b5e-9ac64e70e766/" \
     --subject "system:serviceaccount:<tenant>:<tenant>-postgres" \
     --audience "api://AzureADTokenExchange"
   ```
   (Reusing the same managed identity + storage account with a
   tenant-specific blob prefix is fine for now; provision a dedicated
   storage account per tenant instead if data isolation requirements demand
   it — not needed for the current single-tenant deployment.)
2. Create `deploy/helm/flint-forge/values-<tenant>.yaml` (copy
   `values-flint-forge.yaml`, adjust `backup.azPrefix` to a
   tenant-specific blob prefix).
3. Add a `list` element to `deploy/argocd/flint-forge-applicationset.yaml`
   with that tenant's name/namespace/values file.
4. `kubectl apply -f deploy/argocd/flint-forge-applicationset.yaml` —
   ArgoCD creates the namespace and reconciles the rest.
5. Trigger `deploy-aks.yml` with `tenant: <tenant>` (`workflow_dispatch`) to
   push its first image build, or wait for the next `main` push.

### 13.8 LLM background worker default

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

### 13.9 Deploy to `ssr` (standalone cluster, direct Helm)

**`ssr` is a second, fully separate AKS cluster** — resource group
`sansaba-rg`, subscription `ddefe320-3f3e-45a6-8c68-9c49114af614`, Azure AD
tenant `a4afd01b-3821-4d20-b75c-fad81d76f84d` (`sansabaroyalty`) — not the
`main`/`prometheus-rg` cluster described in §13.1–13.7. Different tenant,
different subscription, different ACR (`sansabaacr.azurecr.io`), and **no
ArgoCD or other in-cluster GitOps controller**. Do not reuse `main`'s Azure
AD app (`github-actions-aks-deploy`) for `ssr` — Azure AD app registrations
are tenant-scoped and cannot be granted access across tenants.

**Deploy mechanism:** `.github/workflows/deploy-ssr.yml` builds images,
pushes to `sansabaacr.azurecr.io`, then runs `helm upgrade --install`
directly against the `ssr` cluster from the runner (`az aks get-credentials`
+ `helm upgrade`). There is no separate "commit triggers reconcile" step —
the workflow run **is** the deploy.

**Shared Gateway (Envoy Gateway, Gateway API).** `ssr` has one pre-existing,
shared `Gateway` object, `acme-http01-gateway` in namespace
`envoy-gateway-system` (labels `app.kubernetes.io/part-of: shared-edge`,
`gateway.sansabaroyalty.com/shared: "true"`), at stable LoadBalancer IP
`64.236.103.210`. Every `*.prometheusags.ai` app on `ssr` attaches
`HTTPRoute`s to this one Gateway rather than creating its own — see
`deploy/helm/flint-forge/templates/gateway-route.yaml`. Onboarding a new
host on `ssr` (out-of-band, not done by this chart):

1. Add a new `https-<name>` listener to `acme-http01-gateway` for the new
   hostname, with `tls.certificateRefs` pointing at a Secret **in the same
   namespace as the Gateway** (`envoy-gateway-system`) — cross-namespace
   secret refs require a `ReferenceGrant` in the Secret's namespace, and the
   pre-existing `https-forge` listener (for the unrelated
   `forge.sansabaroyalty.com`) is broken today specifically because it
   references a Secret in `flint-forge` without one. Do not repeat that
   mistake; keep the cert Secret in `envoy-gateway-system`.
2. Create a `cert-manager.io/v1 Certificate` in `envoy-gateway-system` for
   the new hostname, `issuerRef` the `letsencrypt-http01` `ClusterIssuer`
   (HTTP-01 via `gatewayHTTPRoute`, parented to `acme-http01-gateway`'s
   `http` listener — this is separate from the `letsencrypt-prod`
   ClusterIssuer used by the ingress-nginx path elsewhere on this cluster).
3. Point DNS for the new hostname at `64.236.103.210`.
4. Set `ingress.sharedGateway.httpsSectionName` in the tenant's values file
   to the new listener's name.

`forge-ssr.prometheusags.ai` (this deployment) and `gate-ssr.prometheusags.ai`
(reserved for `flint-gate`) both follow this pattern today, with listeners
`https-forge-ssr` and `https-gate-ssr`.

**`values-ssr.yaml`** (`deploy/helm/flint-forge/values-ssr.yaml`) is the
tenant values file for this cluster: `sansabaacr.azurecr.io` image repos,
`ingress.host: forge-ssr.prometheusags.ai`. `backup` is left at the chart
default (`enabled: false`) — `ssr` has no Workload Identity / wal-g
infrastructure provisioned yet (unlike `main`, see §13.4); set it up the same
way as §13.4 if backups are needed here later.

**Auth — Service Principal with a client secret, not OIDC.** The operator
account in the `sansabaroyalty` Azure AD tenant is a guest with insufficient
privileges to create an App Registration or a federated credential (the
object OIDC login needs — see `az ad app create` /
`az ad app federated-credential create`, both denied with "Insufficient
privileges"). `az ad sp create-for-rbac`, which creates a bare Service
Principal with a client secret rather than a full App Registration, is a
lower-privilege operation and succeeded:

```bash
az ad sp create-for-rbac --name "flint-forge-ssr-deploy" --skip-assignment
# → { appId, password, tenant } — password is shown once, save it immediately

az role assignment create --assignee <appId> --role AcrPush \
  --scope /subscriptions/ddefe320-3f3e-45a6-8c68-9c49114af614/resourceGroups/sansaba-rg/providers/Microsoft.ContainerRegistry/registries/sansabaacr

# ssr has no AAD integration (`az aks show` → aadProfile: null,
# disableLocalAccounts: false) — kubeconfig access is authorized by this ARM
# role, not Kubernetes-level AAD RBAC. "RBAC Writer" (the AAD-integrated
# cluster's role) is a no-op here; use Cluster Admin Role instead.
az role assignment create --assignee <appId> --role "Azure Kubernetes Service Cluster Admin Role" \
  --scope /subscriptions/ddefe320-3f3e-45a6-8c68-9c49114af614/resourceGroups/sansaba-rg/providers/Microsoft.ContainerService/managedClusters/ssr
```

`deploy-ssr.yml`'s `azure/login@v2` step uses the `creds:` JSON form (client
ID + client secret + tenant + subscription) rather than the `client-id`/
`tenant-id`/`subscription-id` trio deploy-aks.yml uses for OIDC — the two are
not interchangeable inputs to the same action. Correspondingly,
`az aks get-credentials` passes `--admin` (fetches the cluster's local admin
kubeconfig, authorized by the Cluster Admin Role above) rather than relying
on AAD-issued kubectl tokens.

Repo config, already set: `vars.AZURE_SSR_CLIENT_ID` / `AZURE_SSR_TENANT_ID`
/ `AZURE_SSR_SUBSCRIPTION_ID`; `secrets.AZURE_SSR_CLIENT_SECRET` /
`SSR_JWT_SECRET` / `SSR_POSTGRES_PASSWORD`; the `production-ssr` GitHub
Environment (no required reviewers yet — add them the same way `production`
gates `main` if that approval step is wanted here too).
