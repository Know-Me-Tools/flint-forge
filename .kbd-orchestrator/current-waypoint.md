# Current Waypoint

- **Phase:** p17-schema-provisioning (FFS-001 — Schema Provisioning API)
- **Status:** execute_ready (assessment + plan complete, both adversarially vetted)
- **Progress:** 0 of 39 tasks across 7 changes (c007 optional, benchmark-gated)
- **Next pending change:** p17-c001-prereqs-auth-migration (Round 1, parallel with c002)
- **Exact next command:** `/kbd-execute p17-schema-provisioning`
- **Backend:** native-kbd (pinned, p16 D-002) — changes under `.kbd-orchestrator/changes/2026-07-30-p17-c00*`
- **Execution rounds:** c001+c002 → c003 → c004 → c005 → c006 (single phase-boundary test run) → c007?
- **Binding notes for execute:** read `phases/p17-schema-provisioning/plan.md`
  → "UNRESOLVED REVIEW FINDINGS" (503-not-404 contract, applied_by=sub,
  D-P1 plan-purity reconciliation). AGENTS.md Integration-First applies:
  tests written per change, suite runs once at the c006 boundary.
- **Known issue:** KBD control plane 401 (documented, deferred) — projection-first state; replay after token reissue.
- **Updated:** 2026-07-30
