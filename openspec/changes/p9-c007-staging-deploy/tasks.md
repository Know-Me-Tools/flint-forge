# p9-c007 Tasks — Staging Deploy

## Tasks

- [x] Create `docker-compose.staging.yml` extending `docker-compose.yml` with resource limits and restart policies
- [x] Create `scripts/smoke_test.sh` — health + functional checks (see proposal); make executable (`chmod +x`) — confirmed executable
- [x] Create `.github/workflows/deploy.yml` — manual trigger, SSH deploy, smoke test
- [x] Add `STAGING_SSH_HOST`, `STAGING_SSH_USER`, `STAGING_SSH_KEY` as GitHub Actions secrets documentation in `docs/runbook.md`
- [x] Create `scripts/README.md` documenting all scripts in `scripts/` — p16-c006 reconcile: found genuinely incomplete (`ci-stack-test.sh`/`verify-migrations.sh` undocumented; a prior pass in this same reconcile had marked it `[x]` without checking every script) — fixed by adding both missing sections rather than just flagging the gap.
- [ ] Test smoke_test.sh locally against running compose stack: `BASE_URL=http://localhost:8080 SMOKE_TOKEN=test ./scripts/smoke_test.sh` — p16-c006: not re-verified in this reconcile pass (would require standing up the full compose stack; out of scope, same reasoning as p9-c001's smoke test). Open debt.
- [x] Verify `docker-compose.staging.yml` is valid: `docker compose -f docker-compose.yml -f docker-compose.staging.yml config` — confirmed just now, exit 0
