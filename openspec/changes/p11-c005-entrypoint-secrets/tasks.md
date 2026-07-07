# p11-c005 Tasks — Dockerfile Entrypoint Secrets Wiring

## Tasks

- [ ] Create `docker/fdb-gateway/entrypoint.sh` — reads `/run/secrets/postgres_password` → sets `DATABASE_URL`; reads `/run/secrets/jwt_secret` → sets `FLINT_JWT_SECRET`; exec binary
- [ ] Make `docker/fdb-gateway/entrypoint.sh` executable (`chmod +x`)
- [ ] Create `docker/fke-server/entrypoint.sh` — reads `/run/secrets/postgres_password` → sets `DATABASE_URL`; exec binary
- [ ] Make `docker/fke-server/entrypoint.sh` executable (`chmod +x`)
- [ ] Update `docker/fdb-gateway/Dockerfile`: add `COPY docker/fdb-gateway/entrypoint.sh /entrypoint.sh`, `RUN chmod +x /entrypoint.sh`, change `ENTRYPOINT` to `["/entrypoint.sh"]`
- [ ] Update `docker/fke-server/Dockerfile`: same pattern
- [ ] Remove `FLINT_JWT_SECRET_FILE` env annotation from `fdb-gateway` service in `docker-compose.prod.yml` (now handled by entrypoint)
- [ ] Add a comment to `docker-compose.yml` base `DATABASE_URL` noting it is overridden by the entrypoint in production when secrets are mounted
- [ ] Validate compose: `docker compose -f docker-compose.yml -f docker-compose.prod.yml config --quiet` (requires stub secret files)
- [ ] `cargo test --workspace` passes (no Rust code changes)
