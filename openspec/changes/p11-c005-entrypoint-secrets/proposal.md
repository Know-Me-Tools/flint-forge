# p11-c005 — Dockerfile Entrypoint Secrets Wiring

**Phase:** 11 — API Stability  **Priority:** P1  **Depends on:** none

## Problem

Both Dockerfiles use a bare binary entrypoint. `DATABASE_URL` and
`FLINT_JWT_SECRET` must be present in the container environment, which currently
requires a `.env` file on the host. The Docker Compose `secrets:` pattern mounts
secret files at `/run/secrets/` but does not automatically translate them into
environment variables — a shell entrypoint wrapper is needed.

## Solution

Add a `sh` entrypoint script to each image that reads the secret files before
exec-ing the binary. This eliminates the `.env` requirement on production hosts.

### `docker/fdb-gateway/entrypoint.sh`

```sh
#!/bin/sh
# Flint Quarry (fdb-gateway) container entrypoint.
# Reads Docker secrets and builds env vars before exec-ing the binary.
set -e

# ── postgres_password ────────────────────────────────────────────────────────
if [ -r /run/secrets/postgres_password ]; then
    PG_PASS=$(cat /run/secrets/postgres_password)
    export DATABASE_URL="postgres://flint:${PG_PASS}@db:5432/flint"
fi

# ── jwt_secret ───────────────────────────────────────────────────────────────
if [ -r /run/secrets/jwt_secret ]; then
    export FLINT_JWT_SECRET=$(cat /run/secrets/jwt_secret)
fi

exec /usr/local/bin/fdb-gateway "$@"
```

### `docker/fke-server/entrypoint.sh`

```sh
#!/bin/sh
# Flint Kiln (fke-server) container entrypoint.
set -e

if [ -r /run/secrets/postgres_password ]; then
    PG_PASS=$(cat /run/secrets/postgres_password)
    export DATABASE_URL="postgres://flint:${PG_PASS}@db:5432/flint"
fi

exec /usr/local/bin/fke-server "$@"
```

### Dockerfile updates

In both `docker/fdb-gateway/Dockerfile` and `docker/fke-server/Dockerfile`,
replace the final two lines:

```dockerfile
# Before:
ENTRYPOINT ["fdb-gateway"]

# After:
COPY docker/fdb-gateway/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
ENTRYPOINT ["/entrypoint.sh"]
```

### `docker-compose.prod.yml` updates

- Remove `FLINT_JWT_SECRET_FILE` env annotation from `fdb-gateway` (now handled
  by entrypoint)
- Update `DATABASE_URL` in base `docker-compose.yml` comment to note it is
  overridden by the entrypoint in production

### Validation

```bash
docker compose -f docker-compose.yml -f docker-compose.prod.yml config --quiet
```

No change to `cargo test` or `cargo clippy` — infrastructure-only change.
