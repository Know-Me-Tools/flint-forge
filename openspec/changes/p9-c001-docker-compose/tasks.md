# p9-c001 Tasks — Docker Compose

## Tasks

- [x] Create `docker-compose.yml` with services: `db`, `fdb-gateway`, `fke-server`
- [x] `db` service: build from `images/postgres18/`, named volume `postgres_data`, healthcheck `pg_isready`
- [x] `fdb-gateway` service: build from `docker/fdb-gateway/`, `depends_on: db: condition: service_healthy`, expose port 8080
- [x] `fke-server` service: build from `docker/fke-server/`, expose port 8090
- [x] Create `.env.example` with all required env var keys and documented defaults
- [x] Create `docker-compose.prod.yml`: extends dev compose, adds `deploy.resources.limits`, `restart: unless-stopped`, removes dev-only env vars
- [x] Add `## Local Development` section to `docs/README.md` (or create it): `docker compose up` quick-start
- [ ] Smoke test: `docker compose up -d && sleep 10 && curl -f http://localhost:8080/healthz && docker compose down` — p16-c006: not re-verified in this reconcile pass (no evidence, e.g. CI log or note, that this has ever actually been executed end-to-end; Docker is available in-environment but running a full compose stack was out of scope for a doc-truth reconcile). Flagged as open debt, not rubber-stamped.
- [x] Verify migrations run automatically on gateway startup (already wired via `sqlx::migrate!`)
