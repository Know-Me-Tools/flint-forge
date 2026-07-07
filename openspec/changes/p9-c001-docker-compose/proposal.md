# p9-c001 — Docker Compose (local dev + prod variant)

**Phase:** 9 — Production Hardening
**Priority:** P0
**Depends on:** existing Dockerfiles from p8-c003

## What this change delivers

- `docker-compose.yml` — single-command local dev stack: Postgres 18, fdb-gateway, fke-server
- `docker-compose.prod.yml` — staging-ready variant with resource limits and restart policies
- `.env.example` — all required env vars with documented defaults

## Design

### Service topology

```
db          postgres18 (from images/postgres18/)
fdb-gateway fdb-gateway:latest (from docker/fdb-gateway/)
fke-server  fke-server:latest  (from docker/fke-server/)
```

`fdb-gateway` depends on `db` via healthcheck; `fke-server` depends on `fdb-gateway`.

### Key design decisions

- Migrations run on gateway startup (`sqlx::migrate!` already wired) — no separate migrate container
- DB data persisted in named volume `postgres_data`
- Secrets via env file (`.env`) — never baked into the image
- `fke-server` shares `DATABASE_URL` with gateway (same pool, different schema access)

### `.env.example` keys

```
DATABASE_URL=postgres://flint:flint@db:5432/flint
FLINT_JWT_SECRET=change-me-in-production
KILN_EPOCH_INTERVAL_MS=10
FLINT_RATE_LIMIT_REST=100
FLINT_RATE_LIMIT_GRAPHQL=20
FLINT_DID_RESOLVER_URL=https://did.flint.example.com
FLINT_REKOR_URL=https://rekor.sigstore.dev
```
