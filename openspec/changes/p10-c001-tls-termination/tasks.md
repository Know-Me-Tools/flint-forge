# p10-c001 Tasks — TLS Termination via Caddy

## Tasks

- [ ] Remove `version: '3.9'` key from `docker-compose.yml`, `docker-compose.prod.yml`, `docker-compose.staging.yml` (cosmetic debt from p9)
- [ ] Create `docker/caddy/Caddyfile` with `reverse_proxy` blocks for fdb-gateway and fke-server
- [ ] Add `caddy` service to `docker-compose.prod.yml` with volumes, env vars, and `depends_on`
- [ ] Override `fdb-gateway.ports` and `fke-server.ports` in `docker-compose.prod.yml` to remove public HTTP exposure (`ports: !reset []`)
- [ ] Add `caddy_data` and `caddy_config` named volumes to `docker-compose.prod.yml`
- [ ] Add `FLINT_DOMAIN` and `CADDY_TLS_EMAIL` to `.env.example` with comments
- [ ] Add `docs/runbook.md §10` covering TLS startup, cert renewal, and troubleshooting
- [ ] Validate compose: `docker compose -f docker-compose.yml -f docker-compose.prod.yml config --quiet`
- [ ] `cargo clippy --workspace -- -D warnings` clean (Rust code unchanged; validate compose only)
- [ ] `cargo test --workspace` passes
