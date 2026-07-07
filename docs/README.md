# Flint Forge — Operations Docs

## Quick Start

```bash
cp .env.example .env
# Edit .env: set FLINT_JWT_SECRET to a secure random value
docker compose up
```

After startup, the following endpoints are available:

- `http://localhost:8080/healthz` — Gateway health
- `http://localhost:8090/healthz` — Kiln health
- `http://localhost:8080/openapi.json` — OpenAPI spec

## Production / Staging

Use the prod overlay to add restart policies and resource limits:

```bash
docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d
```

## Services

| Service | Port | Description |
|---|---|---|
| `db` | 5432 | Postgres 18 with pgvector, pgcrypto, pg_net |
| `fdb-gateway` | 8080 | Flint Data Bus — REST / A2A / A2UI / MCP gateway |
| `fke-server` | 8090 | Flint Kiln Engine — skill runtime |
