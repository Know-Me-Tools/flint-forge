# scripts/

Shell scripts for development, CI, and deployment operations.

---

## `check_api_versions.sh`

Verifies that the API version integers embedded in `docs/api/a2ui.md` and
`docs/api/kiln-abi.md` match the corresponding variables in `.env.example`.
Run automatically on every push via `.github/workflows/ci.yml`.

```bash
./scripts/check_api_versions.sh
```

**What it checks:**

| Doc | Line pattern | `.env.example` variable |
|---|---|---|
| `docs/api/a2ui.md` | `**Current version:** \`N\`` | `FLINT_A2UI_API_VERSION=N` |
| `docs/api/kiln-abi.md` | `**Current ABI version:** \`N\`` | `FLINT_KILN_ABI_VERSION=N` |

**When it fails:** The build fails if any pair is out of sync. The error output
shows exactly which files to update and links to `docs/api/versioning.md`.

**How to bump an API version:** Update both the doc line and the `.env.example`
variable in the same commit, then follow the policy in `docs/api/versioning.md`.

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

## `restore_pg_pitr.sh`

Point-in-time restore drill/procedure for the production Postgres data plane
(p16-c008), using `wal-g` base backups + continuously archived WAL. **Must be
run by an operator, not automatically** — it stops the running `db` container,
replaces its data directory with a restored backup, and starts Postgres in
recovery mode. See `docs/runbook.md` §13.4 for the full architecture and
§13.4.3 for the drill requirement and results log.

```bash
# Restore to the latest available backup (prompts for confirmation)
./scripts/restore_pg_pitr.sh --latest

# Restore to a specific point in time
./scripts/restore_pg_pitr.sh --target-time '2026-07-14 03:00:00+00'

# Skip the confirmation prompt (CI / scripted drills only — never in a real incident)
./scripts/restore_pg_pitr.sh --latest --yes
```

**Prerequisites:** `wal-g` S3 credentials provisioned (`docs/runbook.md`
§13.4.2), the `db`/`backup` services already running via
`docker-compose.prod.yml`.

---

## `rotate_staging_jwt.sh`

Rotates the `staging` GitHub Environment's `JWT_SECRET` secret (p16-c008:
renamed from the repo-level `STAGING_JWT_SECRET` so the same secret name works
per-Environment for both `staging` and `production` — see
`docs/runbook.md` §9.1/§13). It also updates `secrets/jwt_secret.txt` locally
so `mint_smoke_token.sh` can produce tokens signed with the same key.

```bash
# Rotate the staging JWT secret (requires gh CLI to be authenticated)
./scripts/rotate_staging_jwt.sh

# Preview what would happen without touching the filesystem or GitHub
./scripts/rotate_staging_jwt.sh --dry-run
```

**What it does:**

| Step | Action |
|---|---|
| 1 | Generates a fresh 32-byte hex random JWT signing key |
| 2 | Writes it to `secrets/jwt_secret.txt` with `chmod 600` |
| 3 | Runs `gh secret set JWT_SECRET --env staging` to update the GitHub secret |

**Prerequisites:** `gh` CLI installed and authenticated with push access to the
repository.

After rotation, restart the staging stack so the gateway loads the new key:

```bash
docker compose -f docker-compose.yml -f docker-compose.staging.yml up -d
```

Then mint a fresh token and re-run smoke tests:

```bash
TOKEN=$(./scripts/mint_smoke_token.sh)
BASE_URL=https://forge.example.com KILN_URL=http://localhost:8090 \
  SMOKE_TOKEN=$TOKEN ./scripts/smoke_test.sh
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

## `verify-migrations.sh`

Migration integrity check. Validates that `migrations/` has a strict, gap-free numeric prefix sequence with no duplicate prefixes — catches the collision class that broke v1.0 boot (e.g. two migrations both claiming prefix `0005`). When `sqlx-cli` and `DATABASE_URL` are present, also runs `sqlx migrate info`.

```bash
./scripts/verify-migrations.sh
# or against a non-default directory
./scripts/verify-migrations.sh path/to/migrations
```

No environment variables required (`DATABASE_URL` is optional, for the extra `sqlx migrate info` check).

---

## `ci-stack-test.sh`

Full-stack integration + k6 regression gate. Starts the local Docker Compose stack (Postgres 18 + extensions + fdb-gateway + fke-server), applies migrations, runs `DATABASE_URL`-gated tests, and runs `perf/k6/regression.js`. Tears the stack down on exit.

```bash
./scripts/ci-stack-test.sh
```

Environment overrides: `FLINT_JWT_SECRET` (default: generated), `DATABASE_URL` (default: `postgres://flint:flint@localhost:5432/flint`), `BASE_URL` (default: `http://localhost:8080`), `KILN_ADMIN_URL` (default: `http://localhost:8090`).

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
