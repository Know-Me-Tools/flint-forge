# p9-c007 Tasks — Staging Deploy

## Tasks

- [ ] Create `docker-compose.staging.yml` extending `docker-compose.yml` with resource limits and restart policies
- [ ] Create `scripts/smoke_test.sh` — health + functional checks (see proposal); make executable (`chmod +x`)
- [ ] Create `.github/workflows/deploy.yml` — manual trigger, SSH deploy, smoke test
- [ ] Add `STAGING_SSH_HOST`, `STAGING_SSH_USER`, `STAGING_SSH_KEY` as GitHub Actions secrets documentation in `docs/runbook.md`
- [ ] Create `scripts/README.md` documenting all scripts in `scripts/`
- [ ] Test smoke_test.sh locally against running compose stack: `BASE_URL=http://localhost:8080 SMOKE_TOKEN=test ./scripts/smoke_test.sh`
- [ ] Verify `docker-compose.staging.yml` is valid: `docker compose -f docker-compose.yml -f docker-compose.staging.yml config`
