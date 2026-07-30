ASSESSMENT: p17-schema-provisioning
Project: flint-forge
Date: 2026-07-30
Codebase baseline: Post-p16 production-remediated workspace (9/9 P0–P3 gates MET, live AKS deploy verified); Forge is a pure reflect-what-exists gateway with zero provisioning surface.
Cross-tool progress: none (phase created today; progress.json is a fresh skeleton)

## Verification method

Every FFS-001 §1 evidence claim was re-verified against this worktree by grep/read,
not trusted from the spec. Two citations in the spec are slightly off and are
corrected below so /kbd-plan does not chase wrong files. The build-health check ran
`cargo check --workspace` from a cold target dir in this worktree.

IMPLEMENTATION STATUS

- **Schema provisioning API surface (`/schema/v1/*`)**: MISSING — grep across
  `crates/` for `/schema`, `/migrate`, `/ddl`, `execute_ddl`, `create_table`
  returns zero route handlers. The full mounted route set in
  `crates/fdb-gateway/src/bootstrap.rs` (~313–478) covers a2ui/a2a/agui/htmx/mcp/
  design_import only. FFS-001 §1 claim confirmed.
- **Domain spec types (`SchemaSpec`, `TableSpec`, `ColumnSpec`, `ColumnType`,
  `IndexSpec`, `PlanId`, `PlanHash`, `Namespace`)**: MISSING — `fdb-domain`'s
  only source file is `src/lib.rs` (`ls crates/fdb-domain/src/` → `lib.rs`;
  `wc -l` → 326 lines, measured 2026-07-30), holding reflection/REST/GraphQL/
  AG-UI types. As a planning precaution independent of the exact count: the
  new type set is large enough that in-place addition risks the 500-line
  BLOCK constraint → plan a directory-module split
  (e.g. `fdb-domain/src/provision/`) from the start.
- **DDL generator (`fdb-app/src/provision/ddl.rs`, pure `generate()`)**:
  MISSING — `fdb-app` has only `a2ui` and `graphql` modules.
- **`SchemaProvisioner` port**: MISSING — `fdb-ports` exposes `DatabaseBackend`,
  `SchemaProvider`, `RestExecutor`, `SqlExecutor`, `GraphQlExecutor`,
  `ChangeStreamSource`, `KetoCheck`. No mutating DDL seam exists.
- **`PgProvisioner` adapter + dedicated pool**: MISSING — no
  `PROVISIONER_DATABASE_URL` anywhere; `.env.example` has neither
  `FLINT_PROVISION_NAMESPACES` nor `PROVISIONER_DATABASE_URL`.
- **Migration `0015`**: MISSING and the number is free — `migrations/` runs
  `0002`…`0014` (no `0001`, confirming the spec's "next free number" note).
  `0013_force_rls.sql` and `0014_service_role_bypassrls.sql` exist as claimed
  (0014 = service_role BYPASSRLS).
- **`flint_provisioner` Postgres role / ledger schema**: MISSING — no reference
  in any migration or SQL file.
- **DDL reflection endpoint (`GET /schema/v1/tables/{s}/{t}/ddl`)**: MISSING —
  `flint_meta.columns(p_schema, p_table)` exists (defined via `extension_sql!` in
  `crates/ext-flint-meta/src/functions.rs:107`, granted to
  `authenticated, anon, service_role` at `:124`) and is consumed only by the
  reflection engine (`crates/fdb-reflection/src/engine.rs:127`). Not exposed
  over HTTP. FFS-001 §1 claim confirmed, with the caveat that the grant already
  includes `anon` — relevant to open question §11.1 (recon surface).
- **Auth plumbing for `require_provisioner`**: DONE (reusable as-is) —
  `forge_identity::verify_and_build` (`crates/forge-identity/src/lib.rs:135`)
  reads `FLINT_GATE_JWKS_URL` / `FLINT_GATE_ISSUER` / `FLINT_GATE_AUDIENCE`
  (audience fails closed by default since p16-c005; JWKS cache is TTL +
  refetch-on-unknown-kid). The admin idiom to copy is
  `require_admin` in `crates/fke-server/src/handlers/admin.rs` (ADMIN_ROLE =
  `"service_role"`, 401/401/403 ladder exactly matching FFS-001 §4.1), built on
  `fdb_auth::rls_from_bearer` (`crates/fdb-auth/src/lib.rs:21`).
- **Transaction-discipline hazard (D7)**: CONFIRMED REAL —
  `crates/fdb-postgres/src/conn.rs` documents that `acquire` opens `BEGIN` for
  the connection lifetime and that an uncommitted write is silently rolled back
  by deadpool recycling while `RETURNING` still reports success. The
  provisioning adapter must own its own pool + explicit COMMIT, as D7 requires.
- **Reflection hot-swap after DDL (D8 premise)**: DONE (exists) — event trigger →
  `pg_notify('meta_runtime')` → `StateManager` (`crates/fdb-reflection/src/state_manager.rs`,
  NOT `fdb-postgres` as FFS-001 cites) recompiles into `ArcSwap<CompiledState>`.
  The route-mount gap is real: `bootstrap.rs:298–300` carries the exact
  "catch-all delegate pattern (future enhancement)" note the spec quotes, and
  `crates/fdb-gateway/tests/mounts_reflection_router.rs` pins the current
  once-at-startup behavior.
- **Identifier safety**: DONE (reusable) — `forge_domain::is_safe_identifier`
  (`crates/forge-domain/src/lib.rs:72`, with `MAX_IDENTIFIER_LEN` and a
  dot-free segment validator) already gates `SET LOCAL ROLE` in
  `fdb-postgres`.
- **Key material (Phase 0 dependency)**: PRESENT-EXTERNAL (verified by direct
  filesystem listing on 2026-07-30, *outside* this repo — not verifiable from
  repo contents alone) — `sansaba-workspace/infra/scripts/generate-keys.mjs`,
  `infra/keys/jwks.json`, `infra/keys/jwt-private.pem`, and `.env.keys` all
  exist in the Sansaba checkout at
  `/Users/gqadonis/Projects/sansaba/San Saba Automation/sansaba-workspace`.
  Presence-on-disk is NOT end-to-end confirmation: FFS-001 task 0.2 (the key
  actually authenticates against a protected Forge route and lands
  `role = "service_role"` in `RlsContext`) remains a mandatory plan step and
  is the Phase 0 gate, so the dependency must be re-verified in the
  implementation environment regardless of this listing.
- **Docs correction target (task 0.3)**: PARTIAL —
  `docs/ANON-SERVICE-ROLE-KEYS.md` already carries the note that
  `forge token mint` signs HS256 via `FLINT_JWT_SECRET` (line 69) and that
  `FLINT_SERVICE_ROLE_KEY` bypasses RLS, but it still presents
  `FLINT_SERVICE_ROLE_KEY` as a Forge-read variable; grep confirms no Forge
  code reads it. `forge token mint` does exist
  (`crates/forge-cli/src/main.rs:376`) — the doc fix is a clarification, not a
  removal.
- **Workspace deps needed by the generator**: DONE — `sha2 = "0.10"` and
  `uuid` are already `[workspace.dependencies]`; no new crate needed for plan
  hashing or plan IDs (no `ulid` — use uuid or a hash-derived ID).

CROSS-TOOL PROGRESS
- NONE — no cross-tool activity recorded; `progress.json` created today by
  kbd-new-phase with 0/0 counters.

SPEC GAP SUMMARY
(the deliverable is 100% greenfield; these are the *notable* gaps and corrections)

- Everything under FFS-001 §4 (API), §5 (data model), §7 (crate placement) is
  net-new. No partial implementation exists anywhere.
- Two spec citations need correction in planning docs: `state_manager.rs` lives
  in `crates/fdb-reflection/`, not `crates/fdb-postgres/`; and
  `flint_meta.columns()` is defined in `ext-flint-meta/src/functions.rs` (pgrx
  `extension_sql!`), not a standalone SQL file.
- FFS-001 §4.4's "granted to `authenticated`" understates: the grant is to
  `authenticated, anon, service_role` — the recon-surface argument for keeping
  the DDL endpoint privileged is *stronger* than the spec states, since `anon`
  can already read column metadata via SQL if it can reach Postgres, but the
  HTTP surface is the boundary that matters here.
- `fdb-domain/src/lib.rs` (326 lines) cannot absorb the new types without
  violating the 500-line BLOCK — plan a `provision/` module split from the
  start rather than discovering it mid-change.
- The auth building blocks (`verify_and_build`, `rls_from_bearer`,
  `require_admin` idiom) are healthier than the spec assumes post-p16:
  audience validation fails closed by default and JWKS rotation
  (refetch-on-unknown-kid) is already implemented — the §9 key-rotation test
  has a real mechanism to exercise.
- `ext-flint-*` crates are workspace-excluded and MUST NOT be touched via
  plain cargo (constraints BLOCK) — migration 0015 goes in `migrations/`
  (sqlx), which is the right place anyway; no pgrx work is needed for v1
  unless the plan chooses to move ledger DDL into the extension (it should
  not).

BUILD HEALTH
- build check: PASS — `cargo check --workspace` exit 0, zero warnings, cold
  target dir in this worktree, `Finished dev profile in 6m 42s` (2026-07-30).
- known violations: NONE observed in touched areas; workspace `[lints]` gate is
  `clippy::pedantic -D warnings` and p16-c009 added `#![deny(missing_docs)]`
  to all 22 library crates — every new public item in this phase must ship
  documented (with `# Errors` sections) or clippy/CI fails.
- test coverage: N/A for the new surface (nothing exists yet). Relevant prior
  art to reuse: `crates/fdb-gateway/tests/rest_rls_isolation.rs` (two-tenant
  isolation against live Postgres — the §9 "test that matters most" has a
  template), `gateway_startup_live_pg.rs`, and `mounts_reflection_router.rs`.
  CI enforces ≥90% coverage on changed crates (p16-c009), which is stricter
  than FFS-001 assumes.

CONSTRAINT CHECK
- AGENTS.md violations: NONE — `AGENTS.md` exists at repo root (93 lines,
  read in full 2026-07-30). It defers canonical rules to `CLAUDE.md` and
  restates the binding development-management policy: Integration-First
  Delivery (implement the entire plan first, no `todo!()` on live paths, no
  port without adapter, no unmounted handler; test at the phase boundary,
  not mid-phase) and Compile Economy (prefer `cargo check`, batch checks,
  `--release` only for production). Nothing is implemented yet so no
  violations exist, but the policy directly shapes p17 execution: the six
  FFS-001 sub-phases should be implemented integration-first with the test
  suite run once at the phase boundary before reflection.
- constraints.md violations: NONE — but three constraints directly shape the
  plan: (1) hexagonal BLOCK means the provisioner pool/composition happens
  only in `fdb-gateway`; (2) 500-line BLOCK forces `routes/schema/` and
  `fdb-domain/src/provision/` splits up front (FFS-001 task 3.2 already
  anticipates this); (3) new-dependency BLOCK is satisfied — sha2/uuid exist.

GOAL PROGRESS
- G1 (declare table via service_role JWT → generated tenant-scoped DDL):
  NOT MET — no API surface exists.
- G2 (plan/apply separation, planHash, drift guard): NOT MET — greenfield.
- G3 (generated tenant RLS closing the 0013 gap): NOT MET — 0013's disclaimer
  is still the operative state; policy template exists only in the spec (and
  proven in the Sansaba replica, outside this repo).
- G4 (audit ledger, migration 0015): NOT MET — 0015 unwritten; number free.
- G5 (caller/database authority split via flint_provisioner): NOT MET — role
  does not exist.
- G6 (namespace allowlist, default-off 503): NOT MET — env var unread anywhere.
- G7 (DDL reflection endpoint): NOT MET — `flint_meta.columns()` exists but has
  no HTTP exposure.
- G8 (status endpoint): NOT MET.
- G9 (auth via existing RS256 keys + require_provisioner gate): PARTIAL —
  verification pipeline, role-gate idiom, and key material all exist and are
  post-p16 hardened; only the `require_provisioner` copy and the Phase 0
  end-to-end confirmation (0.2) remain.
- G10 (additive-only v1): NOT MET — follows from G1–G6.
- G11 (honest restartRequired flag / optional delegate): NOT MET — the mount
  gap is real and pinned by a test; the delegate does not exist.
- G12 (gating tests: injection corpus, D7 commit test, auth matrix with real
  keys, two-tenant isolation, rotation): NOT MET — though `rest_rls_isolation.rs`
  provides a proven two-tenant live-Postgres test harness to adapt.

RISKS / AREAS OF CONCERN (non-sycophancy gate)

1. **The 90% changed-crate coverage gate (p16-c009) is stricter than FFS-001's
   phase gates.** The spec's Phase 2 adapter work (own pool, transactions,
   ledger) is mostly only testable against live Postgres; if CI's coverage
   measurement does not count `DATABASE_URL`-gated integration tests, the
   adapter crate may need substantial unit-level seams (or a documented gate
   concession) — resolve this in /kbd-plan, not mid-implementation.
2. **Plan-store durability (spec §11.2) is not a free choice.** The in-memory
   option breaks the spec's own CI story (plan at build time, apply at deploy
   time — usually different processes). The ledger-backed `status='planned'`
   row is the only option consistent with §12; the plan should just decide it.
3. **`missing_docs` + pedantic on 22 crates** makes the "small, reviewable"
   phase slicing important: every new public type in `fdb-domain`/`fdb-ports`
   carries mandatory doc burden; budget for it in estimates.
4. **KBD control plane remains 401** (documented, deferred) — stage records for
   this phase are projection-only until the operator reissues the token; the
   daemon's event log is frozen at p16/revision 2 and must be replayed.

ASSESSMENT COMPLETE
