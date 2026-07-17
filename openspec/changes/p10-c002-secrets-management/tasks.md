# p10-c002 Tasks — Secrets Management

## Tasks

- [x] Add `secrets/` to `.gitignore`
- [x] Create `scripts/rotate_secrets.sh` — generates `secrets/jwt_secret.txt` and `secrets/postgres_password.txt` via `openssl rand`; `chmod 600`; prints restart instruction
- [x] Make `scripts/rotate_secrets.sh` executable (`chmod +x`)
- [x] Add `secrets:` top-level block to `docker-compose.prod.yml` with file mounts for `jwt_secret`, `postgres_password`, `caddy_tls_email`
- [x] Update `db` service in `docker-compose.prod.yml`: add `POSTGRES_PASSWORD_FILE` env var and `secrets: [postgres_password]`
- [x] Update `fdb-gateway` service in `docker-compose.prod.yml`: add `FLINT_JWT_SECRET_FILE` env var, update `DATABASE_URL` to passwordless form, add secrets mounts — p16-c006 reconcile note: shipped via a different mechanism than literally described — `docker/fdb-gateway/entrypoint.sh` (added in p11-c005) reads `/run/secrets/jwt_secret`/`/run/secrets/postgres_password` and exports `FLINT_JWT_SECRET`/builds `DATABASE_URL` at container start, rather than a compose-level `FLINT_JWT_SECRET_FILE` env var; `secrets: [jwt_secret, postgres_password]` IS mounted. Same net effect (passwordless, secret-driven config), different mechanism.
- [x] Update `fke-server` service: add `postgres_password` secret mount, update `DATABASE_URL` — same entrypoint-script mechanism as above (`docker/fke-server/entrypoint.sh`)
- [x] Annotate migrated vars in `.env.example` with `# MANAGED VIA DOCKER SECRET` comment — p16-c006 reconcile note: `DATABASE_URL` and `FLINT_JWT_SECRET` both carry this annotation; `CADDY_TLS_EMAIL` does not (still open — see below)
- [x] Update `scripts/README.md` to document `rotate_secrets.sh`
- [x] Add secrets rotation procedure to `docs/runbook.md §10`
- [x] Validate compose: `docker compose -f docker-compose.yml -f docker-compose.prod.yml config --quiet`
- [x] `cargo test --workspace` passes (no Rust code changes; compose-only)

## Still-open debt (p16-c006 reconcile, 2026-07-13)

- [ ] Annotate `CADDY_TLS_EMAIL` in `.env.example` with `# MANAGED VIA DOCKER SECRET` — currently appears only as a commented-out example under a generic "TLS / Caddy (production only)" heading, unlike `DATABASE_URL`/`FLINT_JWT_SECRET` which both carry the annotation.

<!-- p16-c006 reconcile (2026-07-13): verified against .gitignore, scripts/rotate_secrets.sh, docker-compose.prod.yml, .env.example, scripts/README.md, docs/runbook.md §10.7, and `docker compose config --quiet`. One genuine gap found and tracked above rather than rubber-stamped. -->
