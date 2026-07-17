ASSESSMENT: p16-production-remediation
Project: Flint Forge
Date: 2026-07-12
Codebase baseline: Workspace compiles clean (`cargo check` + `clippy -D warnings` both pass) and carries a `v1.0.0` API/ABI-freeze tag, but three security-critical defects sit on live request paths and operational hardening is undone.
Cross-tool progress: p15 (5/5 changes) marked complete by prior tools — pgrx stabilization, migration integrity, operator CLI, e2e/perf CI, docs/Helm. None of the p15 goals addressed the defects below.

---

## REMEDIATION PREMISE

The user goal is 100% remediation of the 2026-07-12 audit → an honest
production-ready v1.0 that is a functional, secure **Supabase replacement for
agentic development and AI-agent systems**. To be that, Flint Forge must, at
minimum, match Supabase's baseline guarantees: **per-tenant row isolation over
the auto-generated data API**, **trusted execution of edge functions**, and
**working realtime**. All three are currently broken or absent. This assessment
maps every audit finding to a remediation change and a machine-verifiable gate.

---

## IMPLEMENTATION STATUS (against a production-ready 1.0)

- **Quarry REST/RPC data API**: PARTIAL(BROKEN on security) — Handlers execute
  real, injection-safe SQL, but on the privileged migration-owner pool with **no
  per-request RLS context**. `require_rls` (`crates/fdb-gateway/src/rls_layer.rs:31`)
  authenticates the JWT and attaches `RlsContext` to extensions but never sets
  `SET LOCAL ROLE` / `request.jwt.claims`. Handlers call `q.fetch_one(&state.pool)`
  directly (`crates/fdb-reflection/src/compilers/rest/mod.rs:166`,
  `.../mutations.rs`, `.../rpc.rs:90`); pool origin is the migration owner
  (`crates/fdb-gateway/src/main.rs:88` → `crates/fdb-reflection/src/state_manager.rs:189`).
  → **Cross-tenant read/write over REST.** [→ p16-c001]

- **Quarry GraphQL query/mutation + introspection**: DONE — Delegated to
  `graphql.resolve()` under full RLS via `PgBackend::acquire`; introspection
  merge real. This is the *correct* pattern that REST must adopt.

- **Quarry realtime subscriptions**: STUB(default) — Transport, fail-closed auth,
  and per-event RLS re-query are real, but the default `FabricChangeSource::watch()`
  returns `futures::stream::empty()` (`crates/fdb-realtime/src/lib.rs:116`, OQ-FRF-1).
  Working `listen` adapter exists but is off unless `FLINT_CHANGE_SOURCE=listen`;
  Helm sets it, compose does not. → **Subscriptions silently dead by default.**
  [→ p16-c004]

- **Kiln edge-function execution**: PARTIAL(BROKEN on trust) — WASI-HTTP invoke
  path + fuel/epoch limits are real, but **no signature verification at any
  ingress** (`fke-sign-*` not even a dependency of `fke-server`; manifest has no
  signature field, `crates/fke-domain/src/lib.rs:39`), capability sandbox not
  enforced (`check_capabilities(granted, granted)` no-op,
  `crates/fke-runtime/src/lib.rs:212`; linker grants full WASI to all),
  anonymous invoke bypasses Cedar, admin control plane unauthenticated, and
  content-addressing uses a fake pseudo-hash labeled `sha256:`
  (`crates/fke-registry/src/lib.rs:108`). → **Executes unsigned, unsandboxed
  WASM.** [→ p16-c002, p16-c003]

- **Auth (JWT verify → RlsContext)**: DONE(with gap) — Real JWKS + signature +
  exp/iss verification, fail-closed, no alg-confusion. Gaps: JWKS never refreshes
  (rotation needs restart); audience skipped when `FLINT_GATE_AUDIENCE` unset.
  [→ p16-c005]

- **Anvil pgrx extensions (auth/hooks/llm/vault/meta)**: DONE — Genuinely
  implemented, no security shortcuts; `ext-flint-vault` (real XChaCha20-Poly1305
  AEAD + HKDF + genuine KMS-envelope DEK, never plaintext/SQL-selectable) is the
  strongest crate. Minor: hooks `agui_run` hardcodes `localhost:8080`
  (`crates/ext-flint-hooks/sql/flint_hooks.sql:156`); llm async worker off by
  default. [→ p16-c006, p16-c008]

- **Operational plane (deploy/backup/perf)**: PARTIAL — TLS overlay (Caddy/ACME),
  Docker secrets, Prometheus/Alertmanager, rate limiting, security headers, Helm
  chart all real. Missing: production deploy automation (`deploy.yml` staging-only),
  automated DB backups/PITR (prod compose DB self-described "staging only"),
  captured k6 baselines (`perf/results/` empty). [→ p16-c008]

---

## CROSS-TOOL PROGRESS

- p15-c001…c005: COMPLETED (by antigravity/opencode per progress.json) — Anvil
  stabilization, migrations, `forge-cli`, Postgres integration CI job, docs+Helm.
  Real work, but **orthogonal** to the security defects; the p15 audit did not
  probe RLS/signature/realtime code paths, which is why these defects survived a
  "production readiness" phase.

---

## SPEC GAP SUMMARY (audit finding → remediation change)

- **REST RLS bypass** (§2.2 contract violated): handlers never set the three
  `SET LOCAL` GUCs; run as table owner. → p16-c001 (P0).
- **Unsigned WASM execution** (§4 auth-layer / Cedar gate absent): no signature
  at ingress; fake content hash; permissive cosign. → p16-c002 (P0).
- **Sandbox + authz not enforced**: capabilities ignored; anon bypasses Cedar;
  admin unauthenticated. → p16-c003 (P0).
- **Realtime empty by default** (§3.3): default source emits nothing. → p16-c004 (P0).
- **JWKS no refresh + audience optional**: → p16-c005 (P1).
- **Hardcoded localhost + stale docs + openspec tracker drift**: docs claim RLS
  enforced (`.../rest/mod.rs:120`) and CRUD is `todo!()` (`:62`) — both false and
  actively misleading; p9–p14 checkboxes unchecked while work shipped. → p16-c006 (P1).
- **500-line rule violated** (own BLOCKING constraint): 17 files over limit,
  worst `renderers.rs` 1267 / `main.rs` 990 / `a2ui.rs` 802. → p16-c007 (P1).
- **No prod CD / no backups / no perf floor**: → p16-c008 (P2).
- **VGV gaps**: no coverage gate, no `deny.toml`, `missing_docs` unenforced, 160
  dup deps, 19 unjustified `unsafe`, 47 error-swallow sites. → p16-c009 (P3).

---

## BUILD HEALTH

- build check: PASS — `cargo check --workspace` (exit 0)
- lint: PASS — `cargo clippy --workspace --all-targets -- -D warnings` (exit 0)
- known violations: **17 files exceed the 500-line BLOCKING constraint**
  (constraints.md); fake `sha256` on Kiln live path; `check_capabilities` no-op.
- test coverage: PARTIAL — 503 test fns, but the critical REST tenant-isolation
  path has **no** live-DB test; live-PG tests are `DATABASE_URL`-gated and skip
  silently; no coverage tooling in CI.

---

## CONSTRAINT CHECK

- AGENTS.md / CLAUDE.md violations: **500-line rule** (17 files); the "never
  expose secrets / enforce RLS at every port boundary" §2.2 intent is violated on
  the REST path (not by a leak, but by absent GUC context).
- constraints.md violations: BLOCKING "no file over 500 lines" — 17 hits. The
  WARN item "any change to the JWT GUC injection sequence is load-bearing" is
  directly relevant to p16-c001 (the sequence is not run on REST at all).

---

## GOAL PROGRESS (production-ready 1.0)

- G-P0-1 REST tenant isolation: NOT MET — cross-tenant exposure; no test.
- G-P0-2 Trusted Kiln execution: NOT MET — unsigned WASM runs; fake hash.
- G-P0-3 Enforced sandbox + authz: NOT MET — capabilities ignored; anon bypass.
- G-P0-4 Realtime works by default: NOT MET — empty stream default.
- G-P1 Auth/config/file-size hygiene: PARTIAL — auth strong but no key rotation;
  hardcoded localhost; 500-line violations.
- G-P2 Production ops: PARTIAL — overlay exists; no CD/backups/perf floor.
- G-P3 VGV compliance: NOT MET — no coverage/deny gates; docs/unsafe/deps debt.

**Overall: NOT production-ready. Four P0 blockers stand between the current
state and an honest v1.0.** Encouragingly, every P0 fix has a correct precedent
already in the codebase (GraphQL's `acquire()` RLS path; the written-but-unwired
signer crates; the working `listen` adapter), so remediation is wiring +
tests, not net-new subsystems.

## Effort shape (see RUST_ASSESSMENT.md §10 for detail)

- P0 (c001–c004): ~1.5–2.5 weeks, 1 senior engineer. The security core.
- P1 (c005–c007): ~1–1.5 weeks.
- P2 (c008): ~1–2 weeks (backups/PITR + prod CD dominate).
- P3 (c009): ~1 week, parallelizable.
- **Serial critical path to an honest 1.0: ~4–6 weeks; P0 alone ~2–3 weeks.**

ASSESSMENT COMPLETE
