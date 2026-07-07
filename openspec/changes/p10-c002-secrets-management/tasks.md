# p10-c002 Tasks — Secrets Management

## Tasks

- [ ] Add `secrets/` to `.gitignore`
- [ ] Create `scripts/rotate_secrets.sh` — generates `secrets/jwt_secret.txt` and `secrets/postgres_password.txt` via `openssl rand`; `chmod 600`; prints restart instruction
- [ ] Make `scripts/rotate_secrets.sh` executable (`chmod +x`)
- [ ] Add `secrets:` top-level block to `docker-compose.prod.yml` with file mounts for `jwt_secret`, `postgres_password`, `caddy_tls_email`
- [ ] Update `db` service in `docker-compose.prod.yml`: add `POSTGRES_PASSWORD_FILE` env var and `secrets: [postgres_password]`
- [ ] Update `fdb-gateway` service in `docker-compose.prod.yml`: add `FLINT_JWT_SECRET_FILE` env var, update `DATABASE_URL` to passwordless form, add secrets mounts
- [ ] Update `fke-server` service: add `postgres_password` secret mount, update `DATABASE_URL`
- [ ] Annotate migrated vars in `.env.example` with `# MANAGED VIA DOCKER SECRET` comment
- [ ] Update `scripts/README.md` to document `rotate_secrets.sh`
- [ ] Add secrets rotation procedure to `docs/runbook.md §10`
- [ ] Validate compose: `docker compose -f docker-compose.yml -f docker-compose.prod.yml config --quiet`
- [ ] `cargo test --workspace` passes (no Rust code changes; compose-only)
