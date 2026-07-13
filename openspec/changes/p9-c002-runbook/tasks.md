# p9-c002 Tasks — Runbook

## Tasks

- [x] Create `docs/runbook.md` with 8 sections (see proposal structure)
- [x] §1 Stack Overview: service map, port table, dependency graph
- [ ] §2 Startup Procedure: step-by-step `docker compose up`, migration check, seed verification, smoke test — p16-c006 reconcile correction: 3 of 4 sub-requirements present (`§2.3` covers start-db → start-services → verify gateway/Kiln health → verify migrations applied), but **no seed-verification step exists anywhere in §2** (confirmed: no mention of `scripts/seed_a2ui_components.sql` or component-table population anywhere in `docs/runbook.md`). A prior pass in this same reconcile marked this `[x]` without checking the specific sub-item — corrected here rather than left rubber-stamped.
- [x] §3 Common Errors: minimum 5 errors with symptom/diagnosis/remediation (see proposal table) — 6 present
- [x] §4 Migration Procedure: apply, verify, rollback steps with exact commands
- [x] §5 Rollback Procedure: image tag rollback, DB snapshot restore, blue/green checklist
- [x] §6 On-Call Severity Matrix: P0–P3 with SLAs and escalation paths
- [x] §7 Security Contacts: breach notification procedure (names/channels can be placeholders)
- [x] §8 Monitoring Checklist: what to check first on any incident alert
- [ ] Link `docs/runbook.md` from the project `README.md` (or root `README.md` if it exists) — p16-c006: verified genuinely missing; neither root `README.md` nor `docs/README.md` references `runbook.md`. Left unchecked as open debt rather than fixed here (out of this reconcile change's scope).
