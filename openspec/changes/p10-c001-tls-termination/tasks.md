# p10-c001 Tasks — TLS Termination via Caddy

## Tasks

- [x] Remove `version: '3.9'` key from `docker-compose.yml`, `docker-compose.prod.yml`, `docker-compose.staging.yml` (cosmetic debt from p9)
- [x] Create `docker/caddy/Caddyfile` with `reverse_proxy` blocks for fdb-gateway and fke-server — p16-c006 reconcile note: fdb-gateway's block is active; the fke-server block is present only as a commented-out optional subdomain block, not an active `reverse_proxy`
- [x] Add `caddy` service to `docker-compose.prod.yml` with volumes, env vars, and `depends_on`
- [x] Override `fdb-gateway.ports` and `fke-server.ports` in `docker-compose.prod.yml` to remove public HTTP exposure (`ports: !reset []`)
- [x] Add `caddy_data` and `caddy_config` named volumes to `docker-compose.prod.yml`
- [x] Add `FLINT_DOMAIN` and `CADDY_TLS_EMAIL` to `.env.example` with comments
- [x] Add `docs/runbook.md §10` covering TLS startup, cert renewal, and troubleshooting
- [x] Validate compose: `docker compose -f docker-compose.yml -f docker-compose.prod.yml config --quiet`
- [x] `cargo clippy --workspace -- -D warnings` clean (Rust code unchanged; validate compose only)
- [x] `cargo test --workspace` passes

<!-- p16-c006 reconcile (2026-07-13): verified against real artifacts (Caddyfile, docker-compose.prod.yml, .env.example, docs/runbook.md §10, `docker compose config --quiet`). All items confirmed done; one nuance noted inline (fke-server's reverse_proxy block is documented-but-disabled, not active). -->
