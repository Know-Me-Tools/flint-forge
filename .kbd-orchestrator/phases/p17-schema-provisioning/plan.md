PLAN: p17-schema-provisioning
Project: flint-forge
Date: 2026-07-30
OpenSpec available: YES at root, but **backend pinned to native-kbd** by p16
decision D-002 (`project.json → change_backend`) — changes are emitted under
`.kbd-orchestrator/changes/`, tasks tracked in `tasks.json`, exactly as p16.
Changes to implement: 7 (6 required + 1 optional)

Source spec: `phases/p17-schema-provisioning/FFS-001-spec.md` (§8 is the seed
breakdown; this plan re-slices its six sub-phases into KBD changes and bakes in
the decisions the assessment flagged as "decide now, not mid-implementation").

DECISIONS BAKED INTO THIS PLAN (not left open)

- **D-P1 Plan store is ledger-backed** (`provision_ledger` rows with
  `status='planned'`, 24h expiry enforced at read time). The in-memory option
  breaks FFS-001's own CI story (plan at build time, apply at deploy time —
  different processes) and §12 requires plan.json to be PR-reviewable across a
  deploy. Resolves FFS-001 §11.2. **Purity reconciliation** (the spec calls
  /plan "pure … writes nothing" while §4.3 requires apply to "re-plan from the
  stored spec" — the spec is internally in tension and this plan resolves it):
  "pure" is scoped to *user-visible schema state* — `POST /plan` executes no
  DDL, mutates no user schema, and is idempotent (same spec ⇒ same planHash;
  re-planning upserts by hash rather than duplicating rows). It DOES write
  exactly one `provision_ledger` row (`status='planned'`) — that row is the
  plan store §4.3 depends on, and it lives in `flint_schema`, not in any
  provisionable namespace. The `generate()` function itself stays a pure
  function; persistence happens only in the route layer.
- **D-P2 `tenantScoped: false` stays allowed with `acknowledgeUnscoped: true`**,
  exactly per FFS-001 §6 — refusing it (spec §11.3) would diverge from the spec
  text this phase is contracted to; revisit only with Sansaba input.
- **D-P3 `GET …/ddl` stays behind `service_role`** (spec §4.4 default). The
  assessment strengthened the recon argument: `flint_meta.columns()` is granted
  to `anon` at SQL level, making the HTTP gate the only boundary that matters.
- **D-P4 Module splits from the start**: `fdb-domain/src/provision/`
  (lib.rs is already 326 lines) and `fdb-gateway/src/routes/schema/` — the
  500-line BLOCK constraint is otherwise hit mid-change.
- **D-P5 Coverage-gate reconciliation is an explicit task** (c006/t1): CI's 90%
  changed-crate gate (p16-c009) vs. the adapter's DATABASE_URL-gated tests must
  be reconciled *before* the phase-boundary test run, with a documented
  concession if the measurement excludes integration tests.
- **D-P6 Integration-First (AGENTS.md, binding)**: tests are *written* per
  change but the suite *runs once* at the phase boundary (c006), then failures
  are driven to green. `cargo check -p <crate>` is the only per-change verify.
  Run-count is recorded in progress.json per Base Rule #18.
- **D-P7 c007 (route hot-reload delegate) is optional and last** — FFS-001 §8
  Phase 5's own gate says ship without it if the benchmark regresses. v1 is
  honest with `restartRequired: true`.

CHANGE LIST (ordered)

1. p17-c001-prereqs-auth-migration: Phase-0 prerequisites — key wiring proof,
   migration 0015, env + docs + runbook.
   - Scope: db (migration) | config | docs | test
   - Depends on: NONE
   - Recommended agent: Claude Code
   - Est. complexity: M
   - Complexity score: Medium
   - Model class: medium
   - Customer value: MEDIUM (unlocks everything; no visible surface yet)
   - Details: Write `migrations/0015_flint_schema_provisioning.sql` verbatim
     from FFS-001 §5 (ledger + `flint_provisioner` NOLOGIN role, idempotent,
     not self-enabling). Add `FLINT_PROVISION_NAMESPACES` +
     `PROVISIONER_DATABASE_URL` to `.env.example` and a runbook §"Schema
     provisioning" (enable/disable/rotate, per FFS-001 §10). Correct
     `docs/ANON-SERVICE-ROLE-KEYS.md` (no Forge code reads
     `FLINT_SERVICE_ROLE_KEY`; `forge token mint` HS256 output cannot pass
     `verify_and_build`). Write the DATABASE_URL-gated auth-proof test:
     Sansaba `service_role` key authenticates a protected route and lands
     `role="service_role"` in `RlsContext`; `anon` key lands `role="anon"`;
     verify `FLINT_GATE_ISSUER`/`FLINT_GATE_AUDIENCE` resolve to `flint-forge`
     against the minted keys (adversarial-review WARNING carried from assess).
     Migration idempotency: applies cleanly twice.

2. p17-c002-domain-ddl-generator: Spec types + pure DDL generator with the
   injection-corpus test suite.
   - Scope: fdb-domain | fdb-app (pure code, no database)
   - Depends on: NONE (parallel-safe with c001)
   - Recommended agent: Claude Code
   - Est. complexity: L
   - Complexity score: High
   - Model class: frontier
   - Customer value: HIGH (the load-bearing security artifact, D1/D5)
   - Details: `fdb-domain/src/provision/` — `SchemaSpec`, `TableSpec`,
     `ColumnSpec`, `#[non_exhaustive] ColumnType` (closed 9-type enum),
     `IndexSpec`, `PlanId`/`PlanHash`/`Namespace` transparent newtypes;
     validation (is_safe_identifier on every identifier, reserved-namespace
     refusal before allowlist, `tenant_id` not caller-declarable when
     tenantScoped, default-expression allowlist: literals + now() +
     gen_random_uuid()). `fdb-app/src/provision/ddl.rs` — pure
     `generate(spec, live) -> Result<Plan, PlanError>` emitting, per FFS-001
     D6's full additive set: CREATE SCHEMA IF NOT EXISTS, CREATE TABLE, ADD
     COLUMN (nullable or defaulted only), CREATE INDEX from caller-declared
     `IndexSpec` entries (name, columns[], unique flag — validated
     identifiers, columns must exist in the table spec) plus the generated
     tenant index, tenant RLS emission verbatim from the FFS-001 §5 template
     (four policies + FORCE + grant), diff → operations + noop (a
     caller-declared index that already exists diffs to no-op, matching the
     §4.2 operations example which lists both permit_watch_tenant_idx and the
     caller's permit_watch_api_idx), canonical serialization + sha256
     planHash (stable field order). Tests written now, run at boundary:
     DDL snapshots (including unique and multi-column caller indexes), hash
     stability across field reordering, noop detection, and the full
     injection corpus ("; DROP TABLE", quoted identifiers, unicode
     homoglyphs, `--` comments, nested `$$` — applied to index names and
     index column lists too). All new public items documented (missing_docs
     is deny).

3. p17-c003-provisioner-port-adapter: `SchemaProvisioner` port + `PgProvisioner`
   adapter with its own pool and explicit-commit discipline.
   - Scope: fdb-ports | fdb-postgres
   - Depends on: p17-c001 (ledger/role exist), p17-c002 (ValidatedPlan types)
   - Recommended agent: Claude Code
   - Est. complexity: M
   - Complexity score: High
   - Model class: frontier
   - Customer value: HIGH (D3 privilege split + D7 correctness live here)
   - Details: `fdb-ports`: `SchemaProvisioner` trait (async_trait, thiserror,
     #[non_exhaustive] errors) beside `SchemaProvider`. `fdb-postgres`:
     `PgProvisioner` with its **own** deadpool from `PROVISIONER_DATABASE_URL`
     — never `DatabaseBackend::acquire` (D7: conn.rs's BEGIN-for-lifetime
     silently discards uncommitted writes). Explicit `BEGIN` → statements →
     `COMMIT`; ledger transitions planned→applied|failed with SQLSTATE only,
     the ledger-write API taking `applied_by: &str` (the JWT `sub`, supplied
     by the gateway — the adapter never sees the bearer) and stamping
     `applied_at`, `version_before`, `version_after`;
     ledger-backed plan store (D-P1: insert `status='planned'` at plan-persist
     time, expiry checked on read). Tracing span records plan_id, namespace,
     role — nothing else (constraints BLOCK on logging claims/tenant ids).
     Tests written: D7 regression (table visible on a FRESH connection after
     apply), failed-statement rollback leaves no partial table + failed ledger
     row, allowlist escape attempt fails at the Postgres privilege layer.

4. p17-c004-gateway-schema-routes: `/schema/v1/{plan,apply,status}` routes,
   `require_provisioner`, feature-gate, OpenAPI.
   - Scope: fdb-gateway (composition root)
   - Depends on: p17-c002, p17-c003
   - Recommended agent: Claude Code
   - Est. complexity: L
   - Complexity score: High
   - Model class: frontier
   - Customer value: HIGH (the API clients actually call)
   - Details: `require_provisioner(&HeaderMap)` copying the `require_admin`
     idiom from `fke-server/src/handlers/admin.rs` (401 missing header / 401
     invalid token / 403 non-service_role).
     **Disabled-state contract — always mount, gate in the handler**: the
     `/schema/v1` group is mounted UNCONDITIONALLY in bootstrap.rs (the
     MCP-group idiom: .route×n → .with_state → .merge); every handler first
     checks the parsed `FLINT_PROVISION_NAMESPACES` allowlist and returns
     `503 schema provisioning is not enabled` when it is empty/unset. FFS-001
     task 3.3's literal wording ("feature-gate mounting … 503 when off") is
     self-contradictory — an unmounted route returns 404, not the 503 that
     §4.1 requires — so this plan resolves it in favor of the §4.1 response
     table: mount always, 503 from the handler. A route test pins
     404-vs-503: disabled deployment returns 503 with that exact error body.
     `routes/schema/{mod,plan,apply,status}.rs` split up front (D-P4). Plan flow (`POST /plan`, per FFS-001 §4.2): deserialize
     typed spec (closed enums — injection fails here) → validate namespace
     against reserved list then allowlist → introspect live namespace via the
     port → `generate()` → persist one `status='planned'` ledger row (D-P1;
     upsert by planHash, no user-schema writes) → return planId, planHash,
     operations[], full ddl text, warnings[] (incl. acknowledged-unscoped),
     noop, expiresAt(+24h). Apply flow (`POST /apply`): load stored spec by
     planHash (404 unknown / 410 expired) → re-plan against live → compare
     hash (409 on drift) → adapter apply, passing the caller's JWT `sub`
     (from the `RlsContext` returned by `require_provisioner`) so the ledger
     row records `applied_by = sub` and `applied_at` — never the raw bearer
     (goal G4 / Base Rule #18; c003's ledger-write API takes `applied_by` as
     a parameter for this) → response with schemaVersion
     before/after, alreadyApplied (200 on replay via the partial unique
     index), restartRequired:true + restartNote (D8 honesty). Status flow
     (`GET /status`, per FFS-001 §4.5): `enabled` (allowlist non-empty),
     `namespaces[]` (the parsed allowlist), `schemaVersion` (current
     reflection version from the state manager), `lastApply`
     {planId, at, status} = most recent ledger row with status applied|failed,
     null when none — served without touching user schemas so it works before
     any provisioning has happened. OpenAPI docs for the group. Route tests
     written for the full FFS-001 §8 Phase-3 matrix: 401/401/403/503/200-plan/
     200-apply/200-alreadyApplied/409-drift/403-reserved-namespace, driven by
     the real Sansaba keys (not synthetic tokens).

5. p17-c005-ddl-reflection-endpoint: `GET /schema/v1/tables/{schema}/{table}/ddl`.
   - Scope: fdb-gateway | fdb-app (synthesis from flint_meta.columns())
   - Depends on: p17-c004 (route group + auth gate exist)
   - Recommended agent: Claude Code
   - Est. complexity: M
   - Complexity score: Medium
   - Model class: medium
   - Customer value: MEDIUM (unlocks client registerEntityFromSql)
   - Details: Synthesize `CREATE TABLE` text from
     `flint_meta.columns(p_schema, p_table)` (defined in
     `ext-flint-meta/src/functions.rs`, already granted; consumed today only by
     `fdb-reflection/src/engine.rs`). Include rlsEnabled/rlsForced/
     schemaVersion. Behind `require_provisioner` (D-P3). Round-trip property
     test written: generate(spec) → apply → GET ddl → re-parse ≡ original
     spec, for every ColumnType × nullable × default.

6. p17-c006-phase-boundary-tests: Phase-boundary integration/e2e suite run and
   drive-to-green (Integration-First checkpoint).
   - Scope: test | ci
   - Depends on: p17-c001..c005 (everything wired; no todo!() on live paths)
   - Recommended agent: Claude Code
   - Est. complexity: M
   - Complexity score: High
   - Model class: frontier
   - Customer value: HIGH (the feature is not done without the isolation test)
   - Details: t1 = reconcile the 90% changed-crate coverage gate with
     DATABASE_URL-gated tests (D-P5) BEFORE the run. Then the single
     phase-boundary suite run (recorded per Base Rule #18): all tests written
     in c001–c005, plus the two e2e tests that only make sense end-to-end —
     (a) two-tenant RLS isolation on a provisioned table (adapt the proven
     harness in `crates/fdb-gateway/tests/rest_rls_isolation.rs`): two JWTs,
     two tenant_ids, one table, tenant B must not see tenant A's row; and
     (b) key-rotation revocation: re-run generate-keys.mjs against a scratch
     JWKS source, old service_role key gets 401 (JWKS refetch-on-unknown-kid
     shipped in p16-c005 makes this testable). Also assert the provisioned
     table appears in /openapi.json after hot-swap — valid WITHOUT c007
     because `openapi_handler` loads `state_manager.current()` per request
     (`crates/fdb-gateway/src/handlers.rs:36–38`); only REST *routes* are
     restart-bound (D8), never the OpenAPI document. Drive all
     failures to green; subsequent runs are for known-failure fix cycles only.

7. p17-c007-route-hotreload-delegate (OPTIONAL — D-P7): catch-all delegate
   flipping `restartRequired` to false.
   - Scope: fdb-gateway
   - Depends on: p17-c004, p17-c006 (baseline green before touching routing)
   - Recommended agent: Claude Code
   - Est. complexity: M
   - Complexity score: Medium
   - Model class: medium
   - Customer value: MEDIUM (quality-of-life; v1 is honest without it)
   - Details: Replace the once-at-startup `.merge(reflection_router)` with a
     fallback loading `state_manager.current().router` per unmatched request
     via `tower::Service`; preserve `require_rls` on the delegated path;
     benchmark ArcSwap::load + router clone on the unmatched path AND confirm
     no regression on mounted routes (`mounts_reflection_router.rs` pins
     current behavior — update it deliberately, not incidentally).
     **Measurable ship criterion**: p50/p99 latency measured on (a) an
     already-mounted route and (b) a delegated reflection route, before vs
     after, ≥1000 requests each on the same build/host; ship only if mounted-
     route p99 regresses <5% AND delegated-path added overhead is <1ms p99.
     Numbers recorded in verification.md. Otherwise close this change as
     SKIPPED-with-evidence (the recorded numbers) and keep the honest flag —
     FFS-001 §8 Phase 5: "an honest flag beats a slow gateway."

EXECUTION ROUND ORDER
Round 1 (parallel): p17-c001, p17-c002
Round 2: p17-c003
Round 3: p17-c004
Round 4: p17-c005
Round 5: p17-c006  (phase-boundary test run happens here, once)
Round 6 (optional): p17-c007

TRADE-OFFS / EXPLICIT SCOPE CUTS (anti-sycophancy)

- No destructive ops, no RENAME, no type changes (FFS-001 D6) — deferred to
  the reviewed migration path, not this API. Anyone asking for DROP in v1 is
  told no by design.
- ADD COLUMN ships in the generator (c002) as spec'd, but the primary Sansaba
  use case is CREATE TABLE; if c002 runs long, ADD COLUMN diffing may slip to
  a follow-up change WITHIN this phase — it may not silently disappear.
- c007 may legitimately not ship (benchmark gate). The phase is complete
  without it; `restartRequired: true` is the disclosed contract.
- The §11.6 key-minting promotion (forge keygen) and §11.7 short-lived
  provisioning token are OUT of this phase — tracked spec open questions,
  triggered by a second consumer, not this plan.
- KBD control plane is still 401 (documented, deferred): all change tracking
  in this phase is projection-first; typed `prometheus kbd change` records
  must be replayed after the operator reissues the token.

COMMANDS TO RUN
# native-kbd backend (pinned) — change structures created by this plan step:
#   .kbd-orchestrator/changes/2026-07-30-p17-c001-prereqs-auth-migration/
#   .kbd-orchestrator/changes/2026-07-30-p17-c002-domain-ddl-generator/
#   .kbd-orchestrator/changes/2026-07-30-p17-c003-provisioner-port-adapter/
#   .kbd-orchestrator/changes/2026-07-30-p17-c004-gateway-schema-routes/
#   .kbd-orchestrator/changes/2026-07-30-p17-c005-ddl-reflection-endpoint/
#   .kbd-orchestrator/changes/2026-07-30-p17-c006-phase-boundary-tests/
#   .kbd-orchestrator/changes/2026-07-30-p17-c007-route-hotreload-delegate/
# then:
/kbd-execute p17-schema-provisioning

UNRESOLVED REVIEW FINDINGS (adversarial-review round 2 of 2 — max rounds
reached; verdict BLOCK accepted with dispositions per the artifact-mode
contract. Execute stage MUST read this section.)

- [CRITICAL → FIXED IN THIS PLAN] 404-vs-503 disabled-state trap: resolved by
  always-mount + in-handler 503 (c004), with a pinning route test. The
  underlying contradiction is in FFS-001 task 3.3's own wording; the plan
  overrides it in favor of the §4.1 response table.
- [CRITICAL → REFUTED WITH CODE EVIDENCE] "openapi hot-swap assertion needs
  c007": false — `openapi_handler` reads `state_manager.current()` per
  request (`crates/fdb-gateway/src/handlers.rs:36–38`); the assertion in c006
  is valid on the v1 restartRequired:true contract. Judge lacked the source
  file; citation now embedded in c006.
- [CRITICAL → FIXED IN THIS PLAN] `applied_by` ledger attribution: now an
  explicit part of c003 (adapter API takes `applied_by`) and c004 (gateway
  passes JWT `sub`, never the bearer).
- [WARNING → ACCEPTED, WORDING TENSION DOCUMENTED] "plan writes nothing" vs
  ledger-backed plan store: D-P1 reconciles purity as scoped to user-visible
  schema state; `POST /plan` writes exactly one `flint_schema` ledger row and
  no user-namespace object. goals.md's "nothing written" phrasing inherits
  FFS-001 §4.2's shorthand; the operative contract is D-P1. If the executing
  agent finds this unacceptable it must raise a blocker, not silently pick a
  side.

PLAN COMPLETE
