# p9-c002 — Runbook

**Phase:** 9 — Production Hardening
**Priority:** P0
**Depends on:** none

## What this change delivers

`docs/runbook.md` — operator reference for the full Flint Forge stack.

## Structure

```
# Flint Forge — Operations Runbook

## 1. Stack Overview
## 2. Startup Procedure
## 3. Common Errors and Remediation
## 4. Migration Procedure
## 5. Rollback Procedure
## 6. On-Call Severity Matrix
## 7. Security Contacts
## 8. Monitoring Checklist
```

## Section highlights

### §3 Common Errors (minimum 5)

| Error | Symptom | Diagnosis | Remediation |
|---|---|---|---|
| DB connection refused | Gateway exits with "reflection pool connect" | `docker compose ps db` | Restart db service; check volume permissions |
| Migration failed | Gateway startup panic | `docker compose logs fdb-gateway` | Fix migration, `docker compose restart fdb-gateway` |
| JWT key expired / missing | All requests return 401 | Check `FLINT_JWT_SECRET` env | Rotate key, restart gateway |
| Kiln artifact not found | `POST /functions/v1/{name}` returns 404 | Check `flint_kiln.functions` table | Re-register function via `POST /admin/functions` |
| Cedar policy denied | `403` on Kiln invocation | Check `flint_kiln.cedar_policies` | Add permit rule for publisher_did |

### §4 Migration procedure

```bash
# Apply
docker compose exec fdb-gateway /usr/local/bin/fdb-gateway --migrate-only
# or: restart service — sqlx::migrate! runs on startup

# Verify
docker compose exec db psql -U flint -c "SELECT id, description FROM _sqlx_migrations ORDER BY installed_on DESC LIMIT 5;"

# Rollback (no down migrations — restore from snapshot)
docker compose down
# restore postgres_data volume from backup
docker compose up -d
```

### §6 Severity matrix

| Severity | Description | Response SLA | Escalation |
|---|---|---|---|
| P0 | Total outage — all requests failing | 15 min | CTO + on-call eng |
| P1 | Partial outage — one subsystem down | 1 hr | On-call eng |
| P2 | Degraded performance or non-critical feature | 4 hr | Business hours |
| P3 | Minor issue, workaround available | Next sprint | Ticket queue |
