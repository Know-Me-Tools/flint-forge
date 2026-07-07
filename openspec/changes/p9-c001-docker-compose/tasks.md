# p9-c001 Tasks — Docker Compose

## Tasks

- [ ] Create `docker-compose.yml` with services: `db`, `fdb-gateway`, `fke-server`
- [ ] `db` service: build from `images/postgres18/`, named volume `postgres_data`, healthcheck `pg_isready`
- [ ] `fdb-gateway` service: build from `docker/fdb-gateway/`, `depends_on: db: condition: service_healthy`, expose port 8080
- [ ] `fke-server` service: build from `docker/fke-server/`, expose port 8090
- [ ] Create `.env.example` with all required env var keys and documented defaults
- [ ] Create `docker-compose.prod.yml`: extends dev compose, adds `deploy.resources.limits`, `restart: unless-stopped`, removes dev-only env vars
- [ ] Add `## Local Development` section to `docs/README.md` (or create it): `docker compose up` quick-start
- [ ] Smoke test: `docker compose up -d && sleep 10 && curl -f http://localhost:8080/healthz && docker compose down`
- [ ] Verify migrations run automatically on gateway startup (already wired via `sqlx::migrate!`)
