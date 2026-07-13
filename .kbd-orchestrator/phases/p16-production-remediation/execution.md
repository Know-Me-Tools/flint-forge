EXECUTION: p16-production-remediation
Project: Flint Forge
Date: 2026-07-12
Selected backend: openspec
Dispatched to: SELF (Claude Code CLI)
Backend rationale: `openspec/` exists at project root; all 9 changes are already
scaffolded as OpenSpec structures (`proposal.md` + `tasks.md`) from `kbd-plan`.
No other tool is connected to this session (`preferred_execution_agents: []` in
`project.json`), so self-execution via `/kbd-apply` gives inspectable,
task-by-task progress without inventing a native-KBD parallel structure.
Backend entrypoint: `/kbd-apply p16-c001-rest-rls-enforcement` (task-by-task
driver over `openspec/changes/p16-c001-rest-rls-enforcement/tasks.md`)
OpenSpec available: YES
Source plan: .kbd-orchestrator/phases/p16-production-remediation/plan.md

EXECUTION SCOPE

- p16-c001-rest-rls-enforcement: Route REST/RPC through `PgBackend::acquire(rls)`; two-tenant integration test — **dispatching now**
- p16-c002-kiln-supply-chain-trust: Manifest signatures + real content hashing — Round 1, pending
- p16-c003-kiln-sandbox-authz: Capability enforcement + invoke/admin auth — Round 2 (after c002), pending
- p16-c004-realtime-default-delivery: Default change-stream source emits events — Round 1, pending
- p16-c005-auth-hardening: JWKS refresh + mandatory audience — Round 1, pending
- p16-c006-config-truth-tracker-reconcile: Doc/config truth + openspec reconcile — Round 2 (after c001), pending
- p16-c007-file-size-compliance: 17-file split to 500-line limit — Round 3, pending
- p16-c008-production-operations: Prod CD + backup/PITR + perf baselines — Round 3, pending (human/operator required for credentials + drills)
- p16-c009-vgv-quality-gates: Coverage/deny/docs/dep/unsafe hygiene — Round 4, pending

DISPATCH CONTRACTS

- p16-c001-rest-rls-enforcement → SELF (Claude Code CLI), via `/kbd-apply`
  Entry: `/kbd-apply p16-c001-rest-rls-enforcement` — drives
  `openspec/changes/p16-c001-rest-rls-enforcement/tasks.md` one task at a time
  (begin-task → implement → end-task), firing `task:before`/`task:after` and
  the plain-text "Starting/Completed task i of n" signals per task.
  Model class: frontier (per plan.md — multi-file, security-critical, needs
  judgment on pool-lifetime wiring)
  Concrete model: no `model_policy` in `project.json` — frontier fallback
  applies; session is currently running `claude-sonnet-5`
  Model rationale: touches `fdb-gateway`, `fdb-reflection`, `fdb-postgres`, and
  a new migration simultaneously; the correctness bar (tenant isolation) does
  not tolerate a lower-tier model's judgment gaps on pool-lifetime/transaction
  scoping
  Progress file: .kbd-orchestrator/phases/p16-production-remediation/progress.json
  Handoff: Report completion by updating `progress.json`
  (`changes.p16-c001-rest-rls-enforcement.status = "completed"`,
  `changes_completed` incremented) and running the artifact-refiner QA gate
  before archive.

- p16-c002 … p16-c009 → SELF (Claude Code CLI), via `/kbd-apply` per change,
  in round order. Not dispatched this pass — see EXECUTION SCOPE. Same
  Model class / Concrete model resolution rules apply per change per
  `plan.md`'s annotations.

APPROVAL GATES

- p16-c008-production-operations: production credential provisioning and the
  first backup/restore drill require human/operator action — this agent will
  not perform those steps autonomously (see `proposal.md` "Design — requires
  human/ops involvement").
- All P0 changes (c001–c004): the phase's own Definition of Done (goals.md)
  treats these as non-negotiable before any production claim — no approval
  gate to skip them, but a human should review the two-tenant isolation test
  results before treating c001 as closed, given the severity of the defect it
  fixes.

FALLBACK CONDITIONS

- If `/kbd-apply` cannot produce inspectable per-task progress (e.g. OpenSpec
  CLI unavailable in this environment) → fall back to driving
  `openspec/changes/p16-c001-rest-rls-enforcement/tasks.md` manually as a
  native KBD change, updating `progress.json` by hand after each task,
  documenting the fallback here.
- If c001's RLS-pool design (Option a vs. b in `proposal.md`) turns out to
  violate the hexagonal layering rule in `constraints.md` (BLOCKING: no
  adapter crate imported from a domain/app crate) → stop and re-plan the seam
  before continuing; do not weaken the layering rule to make the fix fit.

VERIFICATION REQUIREMENTS

- `cargo check --workspace` (compile-economy default per AGENTS.md — use this,
  not `cargo build`, during iteration)
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- New: `DATABASE_URL`-gated two-tenant REST isolation integration test (this
  change's specific gate, per `assessment.md` / `proposal.md`)
- 3-wait budget applies per AGENTS.md Integration-First Delivery — implement
  the full change end-to-end first, spend waits on genuine integration
  checkpoints, not per-function verification.

PROGRESS LEDGER

- [IN_PROGRESS] p16-c001-rest-rls-enforcement — SELF (Claude Code CLI)
- [PENDING] p16-c002-kiln-supply-chain-trust — SELF
- [PENDING] p16-c003-kiln-sandbox-authz — SELF
- [PENDING] p16-c004-realtime-default-delivery — SELF
- [PENDING] p16-c005-auth-hardening — SELF
- [PENDING] p16-c006-config-truth-tracker-reconcile — SELF
- [PENDING] p16-c007-file-size-compliance — SELF
- [PENDING] p16-c008-production-operations — SELF + Manual (operator)
- [PENDING] p16-c009-vgv-quality-gates — SELF

OUTPUTS

- NONE yet — c001 implementation begins after this artifact is written.

BLOCKERS

- NONE

REFLECTION HANDOFF

- `kbd-reflect` should consume: whether the two-tenant RLS isolation test
  passes and what pool-wiring option (a vs. b from `proposal.md`) was chosen;
  whether the fix required any deviation from `constraints.md`'s hexagonal
  layering rule; wait-count spent against the 3-wait budget; any discovery
  that changes the Round 2–4 ordering (e.g. if c001's pool change also
  affects `fdb-postgres` in a way that widens c002's scope).

EXECUTION READY
