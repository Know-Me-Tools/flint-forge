EXECUTION: p17-schema-provisioning
Project: flint-forge
Date: 2026-07-30
Selected backend: claude-code (native-kbd task surface)
Dispatched to: SELF (this Claude Code session, worktree flint-forge-schema-provisioning-0a3dc2)
Backend rationale: change backend is pinned native-kbd (p16 D-002); the task
surface is `.kbd-orchestrator/changes/2026-07-30-p17-c00*/tasks.json`, which
kbd-apply reads natively. This session authored assessment+plan and holds the
full FFS-001 context; self-execution avoids a lossy handoff. The `prometheus
kbd` typed change/task registration is IMPOSSIBLE this phase (control plane
401, documented deferred) — progress.json/tasks.json projections are the
canonical execution artifact, replayed into the runtime after token reissue.
Backend entrypoint: self-driven per-task loop over tasks.json (kbd-apply
semantics: mark done per task, sync progress.json + waypoint at change
boundaries); per-task hook fires unavailable (hooks lib sourcing blocked by
session permission classifier — recorded, best-effort per skill contract).
OpenSpec available: YES but pinned OFF (native-kbd)
Source plan: .kbd-orchestrator/phases/p17-schema-provisioning/plan.md
(BINDING: read its "UNRESOLVED REVIEW FINDINGS" section before each change)

EXECUTION SCOPE

- p17-c001-prereqs-auth-migration: migration 0015, env/docs/runbook, auth-proof test
- p17-c002-domain-ddl-generator: spec types + pure DDL generator + injection corpus
- p17-c003-provisioner-port-adapter: SchemaProvisioner port + PgProvisioner (own pool, D7)
- p17-c004-gateway-schema-routes: /schema/v1 plan/apply/status + require_provisioner
- p17-c005-ddl-reflection-endpoint: GET tables/{s}/{t}/ddl synthesis
- p17-c006-phase-boundary-tests: single Integration-First suite run + e2e (isolation, rotation)
- p17-c007-route-hotreload-delegate: OPTIONAL, benchmark-gated

DISPATCH CONTRACTS
All changes → SELF (claude-code). Model routing per plan.md annotations:

- p17-c001 → SELF | Model class: medium | Concrete model: claude-fable-5 (session model; no model_policy registry in project.json — session model ≥ class floor) | Rationale: mechanical config/docs/migration + one gated test
- p17-c002 → SELF | Model class: frontier | Concrete model: claude-fable-5 | Rationale: load-bearing security artifact (closed grammar, injection corpus)
- p17-c003 → SELF | Model class: frontier | Concrete model: claude-fable-5 | Rationale: privilege split + transaction discipline (silent-rollback hazard)
- p17-c004 → SELF | Model class: frontier | Concrete model: claude-fable-5 | Rationale: auth surface + drift guard + disabled-state contract
- p17-c005 → SELF | Model class: medium | Concrete model: claude-fable-5 | Rationale: bounded synthesis + property test
- p17-c006 → SELF | Model class: frontier | Concrete model: claude-fable-5 | Rationale: RLS isolation e2e is the phase's decisive test
- p17-c007 → SELF | Model class: medium | Concrete model: claude-fable-5 | Rationale: bounded routing change behind a numeric gate

APPROVAL GATES

- NONE within the phase (all operations are in-repo and additive; migration
  0015 executes only when an operator points a gateway at a database).
- c007 ship/skip decision is numeric (benchmark), not approval-based.

FALLBACK CONDITIONS

- If self-execution stalls on a change > 1 session without inspectable
  progress, fall back to openspec backend via /kbd-apply detect and document
  why (per execute protocol OpenSpec Fallback Rule).

VERIFICATION REQUIREMENTS

- Per change: `cargo check -p <touched crates>` (Compile Economy; batch per
  coherent slice). NO test-suite runs mid-phase (AGENTS.md Integration-First).
- Phase boundary (c006): `cargo test --workspace` + DATABASE_URL/key-gated
  tests + `cargo clippy --workspace -- -D warnings`; run count recorded.
- QA gates per completed change: /refine-validate + /adversarial-review
  --mode diff (skip heuristics: <3 files or docs-only).

PROGRESS LEDGER (final, 2026-07-30)

- [DONE] p17-c001-prereqs-auth-migration — SELF (QA: 3 review rounds, dispositions in verification.md)
- [DONE] p17-c002-domain-ddl-generator — SELF (QA: 3 rounds; drift/collision/empty-index hardening)
- [DONE] p17-c003-provisioner-port-adapter — SELF (QA: 3 rounds; audit-invariant + hash-constrained transitions)
- [DONE] p17-c004-gateway-schema-routes — SELF (QA: 2 rounds; 503-not-404 + allowlist-before-replay)
- [DONE] p17-c005-ddl-reflection-endpoint — SELF (QA: 1 round; PK-order + quoting + 36-combo round-trip)
- [DONE] p17-c006-phase-boundary-tests — SELF (6 recorded runs; final 611 passed / 0 failed / 6 ignored with live PG + real keys)
- [SKIPPED-OPTIONAL] p17-c007-route-hotreload-delegate — per plan D-P7, no benchmark run; restartRequired:true stays the disclosed contract

Archive note: change dirs deliberately NOT moved to changes/archive/ this
session — kbd-reflect consumes the verification evidence in place; archive
after reflection.

OUTPUTS

- migrations/0015_flint_schema_provisioning.sql
- crates/fdb-domain/src/provision/, crates/fdb-app/src/provision/
- crates/fdb-ports (SchemaProvisioner), crates/fdb-postgres (PgProvisioner)
- crates/fdb-gateway/src/routes/schema/, bootstrap merge, OpenAPI
- tests across fdb-app/fdb-postgres/fdb-gateway; run records in progress.json

BLOCKERS

- KBD control plane 401 (documented, deferred): typed change/task registration
  replayed post-token-reissue. Not blocking implementation.

REFLECTION HANDOFF

- Per-change QA + adversarial findings under phases/p17-schema-provisioning/review/
- Phase-boundary run record (count, dates, failures→green) in progress.json
- c007 ship/skip decision with benchmark numbers (if reached)
- The 0013 gap-closure evidence: tenant-isolation e2e green on a provisioned table

EXECUTION READY
