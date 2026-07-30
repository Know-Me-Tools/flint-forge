# Phase Reflection: p17-schema-provisioning

**Project:** flint-forge
**Date:** 2026-07-30
**Phase completion:** 100% of required scope (6/6 required changes; 1 optional change deliberately skipped)
**Changes completed:** 6 / 7 (c007 SKIPPED-OPTIONAL by plan rule D-P7)

## Deltas from the plan (what diverged, first)

The phase shipped, but four things did NOT go as planned, and each forced a
design correction:

1. **FFS-001's own §5 SQL was defective.** `FORCE ROW LEVEL SECURITY` with
   zero policies on `provision_ledger` would have dead-lettered every ledger
   write by `flint_provisioner` — the spec's verbatim migration could never
   have worked. Root cause: the spec was never executed against a live
   Postgres before this phase. Corrective action: 0015 adds a
   provisioner-only policy with the deviation documented in-file.
2. **`CREATE SCHEMA` emission was designed, reviewed twice, and still wrong.**
   FFS-001 §4.2's `create_schema` operation survived the plan's adversarial
   review and three per-change review rounds; only the phase-boundary run
   (run 1, two failures) exposed that `CREATE SCHEMA IF NOT EXISTS` fails
   with 42501 for a role without database-CREATE *even when the schema
   exists* — and D3 forbids granting that. Root cause: §4.2's example
   contradicts §10's operator flow, and every reviewer (human-spec, AI
   producer, AI judge) propagated the §4.2 shape. Corrective action: the
   generator never emits `CREATE SCHEMA`; plan/apply gate on a new
   `schema_exists()` port method with an actionable 409.
3. **The plan hash as spec'd could not detect drift.** §4.3 says "re-plan and
   refuse if the recomputed hash differs," but a hash over the spec alone
   trivially equals its own re-plan. Caught while implementing c004, not by
   any review. Corrective action: the hash covers spec + generated DDL, with
   a regression test (`hash_detects_live_schema_drift`).
4. **Rotation revocation has a latency window nobody had measured.** The
   process-global JWKS cache (600s TTL, not URL-keyed) keeps a rotated-out
   key verifying on a warm gateway until expiry. Found by the rotation
   simulation test failing "the wrong way." Corrective action: runbook §14
   now states it plainly — incident-grade revocation = rotate AND restart.

Two pre-existing test gaps outside p17's scope also surfaced, because the
boundary run was the first-ever live execution of tests that
`main-health-2026-07-29.md` had flagged as never verified:
`rest_router_extraction` (fixture never granted `authenticated` anything)
and `rest_typed_columns_live_pg` (stale exact-SQL expectations contradicting
filters.rs's documented deliberate cast). Both verified pre-existing on
unmodified main before repair, both repaired with evidence.

## Goals

Verified against progress.json, the six-run boundary record, and the review
dirs — not against the plan's expectations.

| Goal | Status | Notes |
|---|---|---|
| G1 declare table via service_role JWT → tenant-scoped DDL | MET | plan/apply cycle green with the real Sansaba key (schema_routes 8/8) |
| G2 plan/apply separation, planHash, drift guard | MET | drift guard is real only because the hash was redefined (delta #3); 409 verified by out-of-band-drift test |
| G3 generated tenant RLS closes the 0013 gap | MET | provisioned_table_isolates_two_tenants: tenant B sees nothing, forged INSERT refused, two-sided |
| G4 auditable ledger | MET | hash-constrained transitions, applied_by=sub, DDL cannot commit without its ledger row (orphan-apply test) |
| G5 flint_provisioner authority split | MET | 42501 containment test in ungranted namespaces; SET LOCAL ROLE constant |
| G6 allowlist, default-off 503 | MET | 503-not-404 pinned; reserved-namespace refusal beats operator allowlist |
| G7 DDL reflection endpoint | MET | 36-combination round-trip; composite-PK order; quoted identifiers |
| G8 status endpoint | MET | enabled/namespaces/schemaVersion/lastApply verified in cycle test |
| G9 real-key auth + require_provisioner | MET | anon=403, garbage=401, no-header=401, iss/aud pinned to flint-forge |
| G10 additive-only v1 | MET | no destructive statement kind exists in the generator; drift refuses toward migrations |
| G11 honest restartRequired | MET | true on every apply response; OpenAPI hot-swap verified live WITHOUT c007 |
| G12 gating tests | MET with one armed exception | injection corpus, D7 fresh-connection, auth matrix, isolation e2e, rotation SIMULATION all green; the real-rotation procedure test is armed (FLINT_OLD_SERVICE_ROLE_KEY) but has no evidence until an operator actually rotates |

## Delivered Changes

- `p17-c001-prereqs-auth-migration` — migration 0015, env contract, runbook §14, key-doc corrections, auth-proof test (by: claude-code)
- `p17-c002-domain-ddl-generator` — closed-grammar types, pure generator, injection corpus (by: claude-code)
- `p17-c003-provisioner-port-adapter` — SchemaProvisioner port, PgProvisioner own-pool explicit-commit adapter (by: claude-code)
- `p17-c004-gateway-schema-routes` — /schema/v1 plan/apply/status, require_provisioner, drift guard (by: claude-code)
- `p17-c005-ddl-reflection-endpoint` — CREATE TABLE synthesis (by: claude-code)
- `p17-c006-phase-boundary-tests` — coverage-gate rewiring, isolation/hot-swap/rotation e2e, six-run boundary record (by: claude-code)
- `p17-c007-route-hotreload-delegate` — SKIPPED-OPTIONAL, no benchmark run, spec preserved for a future session

## Artifact Quality Summary

| Metric | Value |
|---|---|
| Changes with QA (refine-validate) | 6/6 required |
| Adversarial diff-review verdict PASS first round | 0/6 (every change drew findings) |
| Review rounds run | c001:3, c002:3, c003:3, c004:2, c005:1(+fixes), c006:1(+fixes) |
| Findings fixed in code | 15 |
| Findings refuted with recorded evidence | 9 |
| Findings accepted as documented limitations | 4 |
| Suite runs at boundary (Base Rule #18) | 6 (final: 611 passed / 0 failed / 6 ignored) |

### Recurring constraint-violation patterns
- `clippy::too_many_lines` on test fns: 3 occurrences (c003, c004, c006) —
  fmt reflow after python-scripted edits repeatedly pushed test fns over 100
  lines; split helpers each time.
- Review-packet tooling defect (not a code pattern): the diff-mode packet
  builder diffs the WORKING TREE, so with a dirty telemetry file the first
  three "reviews" judged noise — c001/c002's initial PASS verdicts were
  theater until re-run with per-commit diffs. This is an adversarial-review
  skill bug worth fixing upstream.

## Technical Debt

- `restartRequired: true` stands (c007 skipped): REST routes for provisioned
  tables need a gateway restart; OpenAPI/MCP/GraphQL/A2UI are live
  immediately. Delegate + benchmark spec preserved in the c007 change dir.
- Real-rotation evidence pending: `rotated_out_service_role_key_is_refused`
  is armed but requires an actual operator rotation to produce evidence.
- Index/policy diffing is conservative: TableMeta carries no index/policy
  introspection, so existing-table plans re-emit idempotent guards instead
  of diffing to noop (documented fail-closed analysis in ddl.rs).
- `OperationKind::CreateSchema` variant is now dead code in the enum
  (non_exhaustive, harmless) — remove opportunistically.
- KBD control plane 401 (pre-existing, deferred): all p17 stage/change
  records are projection-first and must be replayed into the runtime after
  the operator reissues the token.
- p17 evidence beyond implementation (verification.md checkboxes for c001
  gates run in CI) will only be exercised by the next CI run on this branch.

## Architecture Integrity

- CLAUDE.md/AGENTS.md violations: NONE found — hexagonal layering holds
  (generator pure in fdb-app, DTOs in fdb-domain, adapter in fdb-postgres,
  composition only in the gateway); no unwrap/expect in lib code; no
  logging of claims/bearers/tenant ids (spans carry plan_id/namespace/role
  only); no file over 500 lines.
- Constraint concessions, all justified in-place: `#![allow(clippy::expect_used)]`
  + `dead_code` in test support (repo-wide test convention), one
  `too_many_lines` allow on the apply handler's linear state machine.
- One deliberate spec override recorded three times (plan, code, runbook):
  schema lifecycle is operator-owned; FFS-001 §4.2's create_schema example
  lost to its own §10.

## Cross-Tool Coordination Notes

- Progress tracking: RELIABLE — single tool (claude-code) this phase;
  progress.json/tasks.json updated at every change boundary; run counts
  recorded.
- Handoff quality: CLEAR — plan.md's "UNRESOLVED REVIEW FINDINGS" section
  proved to be the binding-contract mechanism that carried three
  review-derived requirements (503-not-404, applied_by=sub, D-P1 purity
  reconciliation) into implementation without loss.
- Gaps: per-task KBD hooks never fired (hooks lib sourcing blocked by the
  session permission classifier) and typed `prometheus kbd` records were
  impossible (401) — both recorded, neither lost state, but the phase ran
  entirely on projections.

## Lessons Learned

- **A spec's evidence table is not the spec's SQL being correct.** FFS-001's
  §1 claims were verifiably accurate while its §5 migration and §4.2
  operation model contained two would-be-production bugs. Verify claims AND
  execute artifacts.
- **The boundary run catches what reviews structurally cannot.** Both
  generator bugs (FORCE-RLS dead-letter, CREATE SCHEMA 42501) passed every
  static review because all reviewers shared the spec's frame; only live
  Postgres disagreed. Integration-First's "one run at the boundary" was the
  single highest-value quality event of the phase.
- **Adversarial review earns its cost mostly through refutation pressure.**
  0/6 first-round passes, but ~40% of findings were wrong and forcing
  evidence-backed refutations (git show, repo conventions, code citations)
  hardened the record as much as the fixes did.
- **Diff-mode review packets must diff the change, not the working tree** —
  three early verdicts judged telemetry noise. Fix upstream in the
  adversarial-review skill; until then, inject per-commit diffs.
- **Hash design rule:** an idempotency/drift token must cover the OUTPUT of
  the function it guards, not just the input — a spec-only hash always
  equals its own re-plan.
- **A 10-year key's revocation story is the JWKS TTL**, not the rotation
  event; measure and document the window, don't assume the refetch is
  instant.

## Next Phase Focus

Recommended next phase: **p18-sansaba-integration** (or fold into the
Sansaba workspace's own plan) — top priorities:

1. **Consume the API from Sansaba** — wire `provides.entities[]` manifest
   blocks to build-time `/plan` + deploy-time `/apply` (FFS-001 §12's
   Option A), including the CI review of `plan.json` in PRs; the cheaper
   `mini_app_record` path (§12) remains valid to ship first.
2. **c007 delegate benchmark** — the spec sits ready; run the p50/p99
   criterion and either flip `restartRequired` or close it permanently.
3. **Operational follow-through** — merge this branch, let the rewired
   coverage gate run in real CI, perform one real key rotation to arm the
   revocation evidence, and replay the p17 KBD records once the control
   plane token is reissued (§11.6 keygen promotion triggers on the second
   consumer, not a date).

## Context for Next Phase

Use this file plus `execution.md`'s REFLECTION HANDOFF block and the
per-change `verification.md` dispositions as prior context for the next
`/kbd-assess` invocation.
