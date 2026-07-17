PLAN: p16-production-remediation
Project: Flint Forge
Date: 2026-07-12
OpenSpec available: YES
Changes to implement: 9

## Approach note (binding per AGENTS.md Integration-First Delivery)

Within each change below, implement the full end-to-end wiring first — no
`todo!()` left on the live path, no port without an adapter — **then** write the
change's gate-defining integration test as the final verification step. Do not
test line-by-line. The 3-wait budget applies per change; record wait-count in
this phase's `progress.json`.

---

## CHANGE LIST (ordered)

### 1. p16-c001-rest-rls-enforcement
- Scope: db | api (fdb-gateway, fdb-reflection, fdb-postgres, migrations)
- Depends on: NONE
- Recommended agent: Claude Code (multi-file, security-critical, needs judgment on pool-lifetime wiring)
- Est. complexity: L
- Complexity score: High
- Model class: frontier
- Customer value: HIGH
- Details: Route every `fdb-reflection` REST CRUD + `/rpc` handler through
  `PgBackend::acquire(rls)` so the three `SET LOCAL` GUCs run per request. Give
  the reflection compiler a distinct, non-owner RLS-subject pool (not the
  migration-owner pool). Add `FORCE ROW LEVEL SECURITY` to tenant tables in a
  new migration. Gate: two-tenant integration test proves isolation over GET/
  POST/PATCH/DELETE/`/rpc` through the real Axum router.

### 2. p16-c002-kiln-supply-chain-trust
- Scope: all (fke-domain, fke-server, fke-registry, fke-store-fs, fke-sign-did, fke-sign-cosign)
- Depends on: NONE
- Recommended agent: Claude Code (crosses 5 crates, cryptographic correctness)
- Est. complexity: L
- Complexity score: High
- Model class: frontier
- Customer value: HIGH
- Details: Add a signature field to `FunctionManifest`; wire `fke-sign-did`/
  `fke-sign-cosign` into `fke-server` at register and invoke ingress; reject
  unsigned/invalid components. Replace the fake `sha256_hex` pseudo-hash
  (`fke-registry`, `fke-store-fs`) with real `sha2::Sha256`. Replace cosign's
  issuer-substring check with real chain/SCT/OIDC-identity verification. Gate:
  unsigned and tampered components rejected at register + invoke; valid signed
  component runs; tests cover all three.

### 3. p16-c003-kiln-sandbox-authz
- Scope: all (fke-runtime, fke-server, forge-policy)
- Depends on: p16-c002 (same files: fke-server/src/main.rs invoke path, fke-runtime linker — sequencing avoids merge conflict, not a hard logical dependency)
- Recommended agent: Claude Code
- Est. complexity: L
- Complexity score: High
- Model class: frontier
- Customer value: HIGH
- Details: Apply each component's declared `capabilities` in the Wasmtime
  linker instead of granting full WASI unconditionally; fix
  `check_capabilities(granted, granted)` to compare requested-vs-granted; wire
  `flint:host` capability surface behind Cedar. Require auth on
  `/functions/v1/<name>` and `/admin/functions`; fail closed on `caller = None`.
  Gate: anonymous invoke denied; ungranted-capability component denied at
  instantiate; unauthenticated admin register is 401.

### 4. p16-c004-realtime-default-delivery
- Scope: all (fdb-realtime, fdb-gateway, docker-compose*.yml)
- Depends on: NONE
- Recommended agent: OpenCode or Codex (focused, low-ambiguity config + adapter default change)
- Est. complexity: M
- Complexity score: Medium
- Model class: medium
- Customer value: HIGH
- Details: Default the change-stream source to the working `listen` adapter (or
  fail loudly with a 503 + startup error instead of an empty stream). Set
  `FLINT_CHANGE_SOURCE` in `docker-compose*.yml` to match the Helm chart's
  behavior. Gate: subscription integration test receives a real change event on
  default configuration.

### 5. p16-c005-auth-hardening
- Scope: api (forge-identity, fdb-auth)
- Depends on: NONE
- Recommended agent: OpenCode or Codex
- Est. complexity: M
- Complexity score: Medium
- Model class: medium
- Customer value: MEDIUM
- Details: Add JWKS TTL/background refresh (or refetch-on-unknown-`kid`) so key
  rotation doesn't require a restart. Make audience validation mandatory
  (fail closed in production mode when `FLINT_GATE_AUDIENCE` is unset). Gate:
  key rotation picked up without restart in a test; missing/wrong-audience
  token rejected in prod mode.

### 6. p16-c006-config-truth-tracker-reconcile
- Scope: all (ext-flint-hooks SQL, fdb-reflection doc-comments, openspec/changes/p9-p14)
- Depends on: p16-c001 (doc corrections must describe the post-fix RLS behavior, not the pre-fix state)
- Recommended agent: OpenCode or Codex
- Est. complexity: M
- Complexity score: Medium
- Model class: medium
- Customer value: MEDIUM
- Details: Make `ext-flint-hooks` `agui_run` target configurable (remove
  hardcoded `http://localhost:8080`). Correct the false doc-comments in
  `fdb-reflection/src/compilers/rest/mod.rs` (`:62`, `:120`). Reconcile
  `openspec/changes/p9-c*` through `p14-c*` checkboxes against what actually
  shipped. Gate: no hardcoded localhost on a deployable path; stale-doc grep
  returns nothing; checkbox state matches code.
- **Caveat:** the tracker-reconcile sub-task is most accurate once c007–c009
  also land — a partial reconcile now may need a second pass at phase close.
  Treat this as a checkpoint, not a final reconcile.

### 7. p16-c007-file-size-compliance
- Scope: all (fdb-gateway routes, fdb-realtime, fke-runtime, fdb-reflection, fdb-query, forge-cli, ext-flint-vault)
- Depends on: p16-c001, p16-c002, p16-c003, p16-c004 (touches the same files those changes modify: `main.rs`, `rest/mod.rs`, `fke-runtime/src/lib.rs`)
- Recommended agent: OpenCode or Codex (mechanical, many small independent splits — good for parallel worktree isolation per file)
- Est. complexity: L (volume, not difficulty — 17 files)
- Complexity score: Medium
- Model class: medium
- Customer value: LOW (internal; but BLOCKING per constraints.md)
- Details: Split each of the 17 files over 500 lines into directory modules
  (worst: `routes/htmx/renderers.rs` 1267, `main.rs` 990, `routes/a2ui.rs` 802).
  No behavior change. Gate: no `crates/**/*.rs` exceeds 500 lines;
  `cargo check`/`clippy` stay green.

### 8. p16-c008-production-operations
- Scope: all (deploy/, .github/workflows/deploy.yml, docker-compose.prod.yml, perf/)
- Depends on: p16-c001, p16-c004 (k6 baseline must be measured against the fixed RLS + realtime system to be meaningful)
- Recommended agent: Claude Code for architecture (CD pipeline, backup strategy design) + **Manual/human required** for actual cloud credential provisioning and the first production backup/restore drill — this agent does not perform irreversible infra provisioning autonomously
- Est. complexity: L
- Complexity score: High
- Model class: frontier
- Customer value: MEDIUM-HIGH
- Details: Extend `deploy.yml` beyond staging-only. Add automated DB backup +
  PITR (pgBackRest/wal-g, or managed-DB wiring) — stop shipping a "staging
  only" production DB. Capture k6 baselines into `perf/results/`; wire the
  regression floor. Decide and document whether `flint_llm`'s async worker
  ships enabled by default. Gate: a production deploy runs from CI; a
  restore-from-backup drill is documented and tested; `perf/results/` holds
  committed baseline numbers.

### 9. p16-c009-vgv-quality-gates
- Scope: all (CI config, workspace Cargo.toml, all crates for `missing_docs`/`unsafe`)
- Depends on: p16-c001 through p16-c008 (audits the final code state — running earlier would need re-running after each P0/P1/P2 change)
- Recommended agent: OpenCode or Codex, parallelizable across sub-tasks (coverage / deny.toml / missing_docs / dep-dedupe / unsafe-audit / error-swallow review are independent of each other)
- Est. complexity: M
- Complexity score: Medium
- Model class: medium
- Customer value: LOW-MEDIUM (process rigor, long-term velocity — not user-facing)
- Details: Add `cargo llvm-cov` coverage gate (≥90% on changed crates) to CI.
  Add `deny.toml` + `cargo-deny` alongside existing `cargo audit`. Enforce
  `#![deny(missing_docs)]` on library crates with `# Errors` sections. Dedupe
  the 160 duplicate dependency versions. Add `// SAFETY:` justification to all
  19 non-pgrx `unsafe` blocks (re-count first — c002/c003 may add more).
  Review the 47 error-swallow sites on non-test paths. Gate: CI enforces
  coverage + deny; `missing_docs` clean on lib crates; `cargo tree --duplicates`
  materially reduced; every `unsafe` justified.

---

## EXECUTION ROUND ORDER

Round 1 (parallel): p16-c001, p16-c002, p16-c004, p16-c005
Round 2 (parallel): p16-c003 (after c002), p16-c006 (after c001)
Round 3 (parallel): p16-c007 (after c001/c002/c003/c004), p16-c008 (after c001/c004)
Round 4: p16-c009 (after everything)

---

## SYCOPHANCY SELF-CHECK

- **S-02**: The 4–6 week / P0-alone-2–3-week estimate from the assessment is
  carried forward unchanged — not compressed to imply a faster path exists.
- **S-07**: Scope held to exactly the 9 changes from `assessment.md`/`goals.md`;
  no new changes invented.
- **S-03 (trade-offs surfaced, not collapsed)**:
  1. If v1.0 is needed sooner than 4–6 weeks, the honest minimum bar is
     **P0-only** (c001–c004, ~2–3 weeks), with P1–P3 explicitly deferred as
     tracked, owned debt per the phase's own Definition-of-Done escape valve —
     not silently dropped.
  2. c008's backup/PITR provisioning and production credentials are exactly the
     kind of hard-to-reverse, externally-visible action this agent will not
     perform autonomously; that sub-task requires a human/ops engineer in the
     loop and is flagged as such rather than assumed closeable end-to-end by an
     agent.
  3. c006's tracker reconcile is a checkpoint, not final, until c007–c009 land.

Sycophancy detector result: score 0.0 (clean) at `standard` strictness,
`pmpo_plan_phase` domain — see
`.kbd-orchestrator/phases/p16-production-remediation/sycophancy/plan-2026-07-12T140429Z.json`.

---

## COMMANDS TO RUN

/opsx:new p16-c001-rest-rls-enforcement
/opsx:new p16-c002-kiln-supply-chain-trust
/opsx:new p16-c003-kiln-sandbox-authz
/opsx:new p16-c004-realtime-default-delivery
/opsx:new p16-c005-auth-hardening
/opsx:new p16-c006-config-truth-tracker-reconcile
/opsx:new p16-c007-file-size-compliance
/opsx:new p16-c008-production-operations
/opsx:new p16-c009-vgv-quality-gates

PLAN COMPLETE
