# p10-c002 — Secrets Management (Docker Compose `secrets:`)

**Phase:** 10 — Production Launch
**Priority:** P0
**Depends on:** p10-c001 (compose structure stabilised)

## Problem

All secrets (`FLINT_JWT_SECRET`, `POSTGRES_PASSWORD`, `CADDY_TLS_EMAIL`) are
read from `.env` as plain environment variables. Operators copying `.env.example`
ship the `change-me-in-production` placeholder to production. There is no rotation
tooling.

## Solution

Use Docker Compose native `secrets:` to mount secrets as files at
`/run/secrets/<name>` inside containers. Services read secrets from the file
path instead of the environment variable.

### Secrets to migrate

| Secret | Current env var | New secret name | Consumer |
|---|---|---|---|
| JWT signing key | `FLINT_JWT_SECRET` | `jwt_secret` | `fdb-gateway` |
| DB password | embedded in `DATABASE_URL` | `postgres_password` | `db`, `fdb-gateway`, `fke-server` |
| TLS email | `CADDY_TLS_EMAIL` | `caddy_tls_email` | `caddy` (from p10-c001) |

### `docker-compose.prod.yml` additions

```yaml
secrets:
  jwt_secret:
    file: ./secrets/jwt_secret.txt
  postgres_password:
    file: ./secrets/postgres_password.txt
  caddy_tls_email:
    file: ./secrets/caddy_tls_email.txt

services:
  db:
    environment:
      POSTGRES_PASSWORD_FILE: /run/secrets/postgres_password
    secrets: [postgres_password]

  fdb-gateway:
    environment:
      FLINT_JWT_SECRET_FILE: /run/secrets/jwt_secret
      DATABASE_URL: postgres://flint@db:5432/flint   # password from secret file
    secrets: [jwt_secret, postgres_password]

  fke-server:
    environment:
      DATABASE_URL: postgres://flint@db:5432/flint
    secrets: [postgres_password]
```

**Note on `DATABASE_URL`:** PostgreSQL's libpq supports `PGPASSFILE` pointing to
a `.pgpass` file. For the Rust `sqlx` driver, the cleanest approach is to use an
entrypoint script that reads `/run/secrets/postgres_password` and builds the
`DATABASE_URL` before exec. Alternatively, pass the password via the
`PGPASSWORD` environment variable (still env-based but not committed to source).
The simplest correct approach for a Compose-native deploy is a small entrypoint
wrapper script.

### `scripts/rotate_secrets.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail
mkdir -p secrets/
openssl rand -hex 32 > secrets/jwt_secret.txt
openssl rand -hex 16 > secrets/postgres_password.txt
chmod 600 secrets/*.txt
echo "Secrets rotated. Restart services: docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d"
```

Add `secrets/` to `.gitignore`.

### `.env.example` annotations

Mark migrated vars as `# MANAGED VIA DOCKER SECRET — see scripts/rotate_secrets.sh`.

### Runbook `§10` additions

Document: `scripts/rotate_secrets.sh` usage, Docker secrets mount paths, and
quarterly rotation procedure.
