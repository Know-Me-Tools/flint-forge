# Rust Code Assessment Report — Flint Forge

> Methodology: Very Good Ventures Rust Code Assessment (enterprise standards).
> Date: 2026-07-12. Scope: full workspace (`crates/`), non-pgrx + `ext-flint-*`.
> Companion: `.kbd-orchestrator/phases/p16-production-remediation/assessment.md`
> (KBD gap report) and `docs/audits/2026-07-12-production-readiness.md`.
> Build baseline: `cargo check --workspace` and `cargo clippy --workspace
> --all-targets -- -D warnings` both pass (exit 0). Tone below is deliberately
> risk-focused; strengths are noted where they set the correct precedent.

---

## 1) Architecture & Crate Structure — Scalability Challenges

### Workspace & Modularity

The hexagonal layering (`*-domain` → `*-ports` → `*-app` → adapters → interface)
is real and enforced at the Cargo-dependency level via `constraints.md`. Crate
boundaries are clean and single-responsibility; adding a new adapter (e.g. a
fifth `ComponentStore`) requires touching only that crate plus the interface
composition root. This is the strongest structural aspect of the codebase and it
is what makes the remediations below *wiring* problems rather than *redesign*
problems.

**Risk — dead adapter surface.** The four `fke-store-{oci,ipfs,s3,fs}` crates and
both `fke-sign-{did,cosign}` crates compile and are individually tested, but
**none are a dependency of `fke-server`** (`grep` for dependents is empty). The
server uses `PgComponentStore` from `fke-registry` and no signer at all. This is
modularity used to *hide* an integration gap: the trait seams exist, so the
system looks pluggable, but the plugs are not inserted. This complicates review —
a reader sees `fke-sign-cosign` and assumes signing happens.

**Risk — interface crates over the size limit.** `fdb-gateway/src/main.rs` (990
lines) and `routes/htmx/renderers.rs` (1267) violate the project's own 500-line
BLOCKING rule. 17 files total exceed it. This concentrates composition logic and
makes the gateway hard to review as isolated units.

### Dependency Injection

Trait-based DI is used correctly and pervasively: `Arc<dyn DatabaseBackend>`,
`Arc<dyn ChangeStreamSource>`, `Arc<dyn KetoCheck>`, `Arc<dyn Pep>` are injected
at the composition root (`fdb-gateway/src/main.rs`), and tests substitute mocks
(`fdb-app/tests/gate_tests.rs`). Configuration is read from env at the root, not
hardcoded in libraries — with two exceptions that leak into deployable paths:
`ext-flint-hooks` `agui_run` hardcodes `http://localhost:8080`
(`sql/flint_hooks.sql:156`) and `fke-server` hardcodes bind `0.0.0.0:8090`.

---

## 2) Error Handling — Debugging & Reliability Risk

`thiserror` in libraries, `anyhow` at binary edges, `#[non_exhaustive]` on public
enums — the discipline is present and CI-gated. Verification failures fail closed
with `401`/`403` rather than being swallowed.

**Risk — silent error swallowing (47 sites).** `grep` finds 47 `.ok();` / `let _
= …` occurrences on non-test paths. Many are legitimate fire-and-forget, but the
category is exactly where visibility bugs hide (e.g. a manifest that fails to
serialize is stored as SQL `NULL` via `serde_json::to_value(...).unwrap_or(Value::Null)`
in `fke-server/src/main.rs:358` rather than rejected). Each needs a one-line
review pass.

**Risk — `# Errors` rustdoc coverage is inconsistent.** Fallible public functions
in `fdb-postgres`, `fke-registry`, and `fdb-query` frequently lack an `# Errors`
section, so callers cannot see failure modes from the docs. VGV mandates this.

**Positive.** No `todo!()`/`unimplemented!()`/`panic!` on any live request
handler. `unwrap()`/`expect()` on request paths are confined to `#[cfg(test)]`;
startup `expect()`s at the composition root are acceptable fail-fast.

---

## 3) Type Safety & Correctness — Maintenance Deficits

**Newtypes.** `#[repr(transparent)]` ID newtypes are established in
`forge-domain` and used consistently — good. Some `String`-typed domain fields
remain (e.g. `FunctionManifest.publisher_did`, `content_digest`,
`fke-domain/src/lib.rs:39`) where a `Did` / `Digest` newtype would prevent the
digest-mismatch hazard between register and invoke.

**Correctness defect — fake content hash.** `fke-registry/src/lib.rs:108`'s
`sha256_hex` is an FNV-style pseudo-hash labeled `sha256:` with a self-admitting
`TODO`. `fke-store-fs` has the same. On the live path this means content-
addressing provides **no integrity and is trivially collidable** — a correctness
and security defect, not cosmetic.

**Correctness defect — tautological capability check.**
`check_capabilities(granted, granted)` (`fke-runtime/src/lib.rs:212`) passes the
same argument twice and can never fail; the declared-vs-granted comparison it
implies does not happen.

**Unsafe.** 19 `unsafe` blocks in non-pgrx crates (Wasmtime host integration,
FFI) + pervasive `unsafe` in `ext-flint-*` (pgrx, idiomatic). Most lack a
`// SAFETY:` justification comment. VGV/Rust-API-guideline requires each to
document its invariant.

**Pattern matching.** Exhaustive matching is the norm; `#[non_exhaustive]` is
applied. No systemic `_ => {}` catch-all risk found.

---

## 4) Async & Concurrency — Safety & Performance Risk

**Positive.** Tokio usage is disciplined: `#[tracing::instrument]` across port
boundaries, `BoxStream` subscriptions, bounded broadcast channel in the `listen`
adapter (capacity env-tunable, default 1024), `tower_governor` rate limiting,
and — notably good — `ext-flint-llm` runs the synchronous LLM call on an isolated
thread with a hard GUC timeout so a hung upstream cannot wedge a Postgres
backend. Wasmtime fuel + epoch-deadline interruption are genuinely enforced with
a background epoch ticker.

**Risk — fragile trap classification.** Epoch-trap detection uses
`e.to_string().contains("epoch")` (`fke-runtime/src/lib.rs:374`). A Wasmtime
error-message change silently reclassifies timeouts. Match on the typed trap
instead.

**Risk — the default realtime path is a no-op, not a slow path.**
`FabricChangeSource::watch()` returns `stream::empty()` — not a concurrency bug
per se, but a data-plane behavior that looks live (auth runs, stream opens) and
delivers nothing. This is the kind of "succeeds emptily" async surface VGV warns
about.

---

## 5) Testing — Barrier to Expansion & Refactoring

503 test functions exist and much is genuine (pgrx `#[pg_test]` suites for
Anvil, `wiremock` for signers, `testcontainers` for stores).

**Risk — the highest-severity path has zero coverage.** There is **no test that
proves REST tenant isolation** through the real Axum router. The one live REST
test (`fdb-postgres` `pgrest_live_pg.rs`) exercises the *non-mounted* executor
and explicitly uses `current_user` as the role, stating the test is about the
query builder, "not RLS policy enforcement." The mount test uses a lazy pool and
still carries stale `todo!()` assumptions. This coverage hole is precisely why
the RLS bypass shipped through a "production readiness" phase.

**Risk — live tests skip silently.** 11 `DATABASE_URL`-gated tests return early
when the env var is unset; 7 are additionally `#[ignore]`. Default `cargo test`
exercises none against a DB, so green local runs prove little about integration.
CI does run a Postgres integration job (`ci.yml`), which mitigates this in CI
only.

**Risk — no coverage tooling.** No `cargo llvm-cov`/`tarpaulin` in CI; no
coverage threshold. VGV mandates a coverage gate; there is none, so refactoring
is high-risk (you cannot see what a change stops covering).

**Missing.** No property/fuzz tests on the SQL filter/operator translator
(`fdb-query`) or on serde boundaries — both high-value `proptest` targets given
they sit on the injection-safety boundary.

---

## 6) CI/CD & Tooling — Process Gaps

**Present.** `.github/workflows/ci.yml` runs `cargo fmt --all --check`, `cargo
clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
`cargo audit`, and a Postgres-18 integration job. Conventional Commits and a
maintained `CHANGELOG.md` (git-cliff) are in place. This is a solid baseline.

**Gaps vs VGV:**
- **No coverage gate** — `cargo llvm-cov` absent; VGV requires coverage on every PR.
- **No `deny.toml` / `cargo-deny`** — license + advisory policy is unmanaged
  (only `cargo audit` for advisories).
- **No `clippy.toml` / `rustfmt.toml`** — team conventions are implicit.
- **No spell-check** step (VGV runs one).
- **Deploy automation is staging-only** (`deploy.yml`); no production pipeline.
- **openspec tracker drift** — p9–p14 `tasks.md` checkboxes are near-universally
  unchecked while the work shipped; a reviewer using the tracker to gauge
  readiness is actively misled. This is a process-integrity defect, not just
  bookkeeping.

---

## 7) API Design & Documentation — Onboarding Cost

**Risk — actively false doc-comments.** `fdb-reflection/src/compilers/rest/mod.rs:62`
says "CRUD handlers remain `todo!()` stubs" (they are fully implemented) and
`:120` says "RLS is enforced by the connection's GUC context" (no GUC context is
set on that pool). Documentation that contradicts the code is worse than none —
it is what masked the RLS defect from reviewers. These must be corrected as part
of remediation, not deferred.

**Risk — `missing_docs` unenforced.** Only one crate declares
`deny/warn(missing_docs)`. Public items across library crates lack `///` docs;
`#[must_use]` is inconsistently applied on `Result`-returning constructors.

**Positive.** `README.md`, `docs/runbook.md` (1093 lines, detailed), `docs/
security-audit.md`, `docs/performance.md`, and `docs/ROADMAP.md` exist and are
substantive. The runbook is genuinely operator-grade. Onboarding cost for *ops*
is low; onboarding cost for *library API* is raised by thin rustdoc.

---

## 8) Dependencies — Supply Chain & Bloat Risk

**Risk — 160 duplicate dependency versions** (`cargo tree --duplicates`). This
inflates compile time and binary size and widens the advisory surface. Needs a
dedupe pass (align versions via `[workspace.dependencies]`, which the project
already centralizes on).

**Positive.** `Cargo.lock` is committed; MSRV declared (`1.85`, toolchain `1.90`
pinned); `cargo audit` runs in CI; a recent RUSTSEC advisory (crossbeam-epoch)
was already remediated per `CHANGELOG.md`. The split pgrx versions (`0.12` pg17
for auth, `0.18.1` pg18 for vault) are intentional and documented.

**Gap.** No `cargo-deny` means license compliance and yank/ban policy are
unchecked — relevant for a project positioning as a Supabase replacement others
will vendor.

---

## 9) Performance & Resource Management — Production Readiness

**Positive.** deadpool/sqlx connection pooling; `tracing` structured logging with
spans across port boundaries; Prometheus `/metrics` with sqlx pool gauges;
Alertmanager rules and a Grafana dashboard shipped; network calls (JWKS, Keto)
carry timeouts; the vault zeroizes transient key material.

**Risk — no measured performance floor.** k6 scripts exist (`perf/k6/*.js`) but
`perf/results/` is empty (`.gitkeep` only); the regression gate is
`workflow_dispatch`-only and was **deferred** at v1 because staging was
unavailable. There is no baseline, so "production performance" is an unverified
claim.

**Risk — no automated backup / PITR.** `docker-compose.prod.yml` self-describes
its DB as "staging only; prefer a managed database"; backup is a manual
`pg_dump` procedure in the runbook. For a data plane, absence of tested restore
is a production blocker.

**Risk — JWKS cached for process lifetime** (`OnceLock`, no TTL). Upstream key
rotation is not picked up until restart — a resource/lifecycle gap on the auth
hot path.

---

## 10) Refactoring Estimation & Summary

### Top 3 Risks

1. **REST tenant-isolation bypass (data-plane security).** Every REST CRUD +
   `/rpc` call runs on the migration-owner pool with no RLS GUC context; an
   authenticated user of tenant A can read/mutate tenant B's rows. This alone
   disqualifies a "Supabase replacement" claim, whose core promise is
   RLS-enforced auto-APIs. (`fdb-reflection/src/compilers/rest/`,
   `fdb-gateway/src/rls_layer.rs`, pool origin `main.rs:88`.)

2. **Untrusted Kiln execution (compute-plane security).** No signature
   verification at any ingress, capability sandbox not enforced, anonymous invoke
   bypasses Cedar, admin plane unauthenticated, fake content hashing. The runtime
   will execute arbitrary unsigned WASM with full WASI. (`fke-server/src/main.rs`,
   `fke-runtime/src/lib.rs:212`, `fke-registry/src/lib.rs:108`.)

3. **Process integrity: docs and trackers assert states the code does not hold.**
   Doc-comments claim RLS is enforced and CRUD is stubbed (both false); openspec
   checkboxes under-report shipped work; the realtime default succeeds emptily.
   These make the system look more done and more secure than it is, which is how
   the P0 defects survived a prior readiness phase.

### Refactoring Scope (key tasks → mapped changes)

- Route REST/RPC handlers through `PgBackend::acquire(rls)`; add a non-owner
  RLS-subject pool; `FORCE ROW LEVEL SECURITY`; two-tenant HTTP integration test. → **p16-c001**
- Add manifest signatures; wire `fke-sign-*` into `fke-server`; real
  `sha2::Sha256`; real cosign chain/SCT verification. → **p16-c002**
- Enforce declared capabilities in the Wasmtime linker; fix `check_capabilities`;
  require auth on invoke + admin; fail closed on `caller = None`. → **p16-c003**
- Default realtime to a source that emits (or fail loud); set
  `FLINT_CHANGE_SOURCE` in compose. → **p16-c004**
- JWKS TTL/refresh; mandatory audience. → **p16-c005**
- Configurable `agui_run`; correct false doc-comments; reconcile openspec. → **p16-c006**
- Split 17 over-limit files into directory modules. → **p16-c007**
- Prod deploy CD; automated backup/PITR + tested restore; k6 baselines. → **p16-c008**
- Coverage gate; `deny.toml`; `missing_docs`; dep dedupe; `unsafe` justification;
  error-swallow review. → **p16-c009**

### Time Estimates

#### Minimal (Critical Gaps): ~2–3 weeks
The four P0 changes (c001–c004) plus their integration tests, one senior Rust
engineer. Every fix has a correct precedent already in-tree (GraphQL's `acquire()`
RLS path; the written-but-unwired signer crates; the working `listen` adapter),
so this is disciplined wiring + tests, not new subsystems.

#### Comprehensive (Full Compliance / honest v1.0): ~4–6 weeks
Adds P1 (auth rotation, config/doc truth, 500-line splits), P2 (prod CD +
backups/PITR + measured perf floor), and P3 (coverage + deny + docs + dep dedupe
+ unsafe audit). P2's backup/PITR and prod pipeline dominate the tail.

### Justification

Fixing P0 is not optional polish: without it the product fails its two defining
promises (tenant-isolated data APIs, trusted edge functions) and would leak
cross-tenant data and execute unsigned code on day one. The architecture is
sound and the correct patterns already exist beside the broken ones, so the
refactor is bounded and low-redesign-risk — but it must be gated by the
integration tests that are currently missing, or the same class of defect will
recur. Correcting the false documentation and tracker state in the same pass is
what prevents a future phase from again mistaking "compiles and unit-tests pass"
for "production-ready."
