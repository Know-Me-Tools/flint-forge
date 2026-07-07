# scripts/

Shell scripts for development, CI, and deployment operations.

---

## `rotate_secrets.sh`

Generates Docker secret files for the production stack and keeps `.env` in sync.
Run this before the first deploy and on every quarterly rotation cycle.

```bash
# Interactive (prompts for ACME email if CADDY_TLS_EMAIL is not in the environment)
./scripts/rotate_secrets.sh

# Non-interactive (all values from environment)
CADDY_TLS_EMAIL=ops@example.com ./scripts/rotate_secrets.sh

# Dry run — show what would be written without touching the filesystem
./scripts/rotate_secrets.sh --dry-run
```

**What it generates:**

| File | Content | Used by |
|---|---|---|
| `secrets/jwt_secret.txt` | 32-byte hex random string | `fdb-gateway` (mounted at `/run/secrets/jwt_secret`) |
| `secrets/postgres_password.txt` | 16-byte hex random string | `db` (`POSTGRES_PASSWORD_FILE`), all app containers |
| `secrets/caddy_tls_email.txt` | ACME email address | `caddy` (mounted at `/run/secrets/caddy_tls_email`) |

The script also updates `DATABASE_URL` in `.env` to use the newly generated
password so the app containers stay in sync.

**Security:** All secret files are created with `chmod 600`. The `secrets/`
directory is `chmod 700`. The directory is gitignored — never commit secret
files.

After rotation, restart the production stack:

```bash
docker compose -f docker-compose.yml -f docker-compose.prod.yml \
  up -d db fdb-gateway fke-server caddy
```

---

## `ci-check.sh`

Canonical quality gate — runs `rustfmt --check`, `cargo clippy --workspace -- -D warnings`, and `cargo check`. Executes identically locally and in CI.

```bash
./scripts/ci-check.sh
```

No environment variables required.

---

## `ci-test.sh`

Two-stage test runner:

1. **Unit stage** — always runs; no database required.
2. **DB-integration stage** — runs only when `DATABASE_URL` is set. Applies migrations, then runs the full test suite.

```bash
# Unit tests only
./scripts/ci-test.sh

# Full suite with database
DATABASE_URL=postgres://flint:flint@localhost/flint ./scripts/ci-test.sh
```

The database must have `vector` (pgvector) and `pg_graphql` available — use the pinned Postgres image in `images/postgres18/`.

---

## `smoke_test.sh`

Post-deploy smoke checks. Validates that fdb-gateway and fke-server are responding correctly after a `docker compose up`.

```bash
# Unauthenticated checks only (no JWT required)
BASE_URL=http://localhost:8080 KILN_URL=http://localhost:8090 \
  ./scripts/smoke_test.sh

# Full checks including authenticated endpoints
BASE_URL=http://localhost:8080 KILN_URL=http://localhost:8090 \
  SMOKE_TOKEN=<jwt> ./scripts/smoke_test.sh
```

**Environment variables:**

| Variable | Default | Description |
|---|---|---|
| `BASE_URL` | `http://localhost:8080` | fdb-gateway base URL |
| `KILN_URL` | `http://localhost:8090` | fke-server base URL |
| `SMOKE_TOKEN` | *(empty)* | JWT bearer token; authenticated checks are skipped when absent |
| `TIMEOUT` | `10` | curl connect+max-time in seconds |

**Checks performed:**

| Endpoint | Auth | Expected |
|---|---|---|
| `GET /healthz` | none | 200 |
| `GET /openapi.json` | none | 200 |
| `GET /metrics` | none | 200 |
| `GET /a2ui/v1/components` | Bearer | 200 |
| `GET /mcp/v1/tools` | Bearer | 200 |
| `GET /a2ui/v1/components` | none | 401 (auth guard active) |
| `POST /functions/v1/__smoke_nonexistent__` | none | 4xx (kiln alive + gating) |

Exit code `0` = all checks passed. Exit code `1` = one or more failures.

---

## `mint_smoke_token.sh`

Mints a self-signed HS256 JWT for use in smoke tests. Outputs a single JWT
string to stdout. The token expires 1 hour after minting.

```bash
# Explicit key
JWT_SECRET=mysecret ./scripts/mint_smoke_token.sh

# Reads secrets/jwt_secret.txt automatically (after rotate_secrets.sh)
./scripts/mint_smoke_token.sh

# Capture the token for use in another command
TOKEN=$(JWT_SECRET=mysecret ./scripts/mint_smoke_token.sh)
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/graphql
```

**Signing key resolution (first match wins):**

1. `$JWT_SECRET` environment variable
2. `secrets/jwt_secret.txt` (local dev / staging host)
3. `/run/secrets/jwt_secret` (inside a container)

**Environment variables:**

| Variable | Required | Description |
|---|---|---|
| `JWT_SECRET` | optional | Raw HS256 signing key; overrides file lookup |

**Output format:**

A single newline-terminated string with three base64url-encoded segments
separated by `.`:

```
<header>.<payload>.<signature>
```

The payload contains `sub`, `role`, `exp`, and `iat` claims:

```json
{
    "sub": "smoke",
    "role": "authenticated",
    "exp": 1700000000,
    "iat": 1699996400
}
```

**Verify the output:**

```bash
JWT_SECRET=test123 ./scripts/mint_smoke_token.sh | \
  cut -d. -f2 | \
  awk '{n=length($0)%4; if(n>0) for(i=n;i<4;i++) printf "="; print}' | \
  base64 -d 2>/dev/null | python3 -m json.tool
```

**Dependencies:** `openssl`, `base64`, `tr`, `date` — all standard on
macOS and Debian/Ubuntu. No external tools required.

---

## `seed_a2ui_components.sql`

One-time SQL seed script that populates `flint_a2ui.components` with the 55 base A2UI component records. Run this against a fresh database when `USE_SEED=true` is not set at startup, or when you need to reset the component catalogue.

```bash
psql "$DATABASE_URL" -f scripts/seed_a2ui_components.sql
```
