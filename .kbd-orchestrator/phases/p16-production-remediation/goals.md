# Goals — p16-production-remediation

## Phase Summary

Remediate **100%** of the 2026-07-12 critical production-readiness audit so
Flint Forge can be **honestly** declared a production-ready v1.0 — a functional,
secure Supabase replacement for agentic software development and AI-agent
systems.

The existing `v1.0.0` tag is an **API/ABI contract freeze**, not an operational
milestone. The prior phase `p15-v1.0-production-readiness` was marked complete
but its five goals (pgrx stabilization, migrations, CLI, e2e/perf, docs) never
covered the three security-critical defects on live request paths. This phase
closes that gap.

Seeded from: user directive (2026-07-12) + `docs/audits/2026-07-12-production-readiness.md`

---

## Non-negotiable acceptance criterion

**No production claim is valid while any P0 change is open.** A tenant can read
another tenant's data over REST today; the Kiln runtime executes unsigned WASM
today; subscriptions deliver nothing by default today. These are the gate.

---

## Changes (9 planned)

### P0 — Security-critical; block ANY production claim

- **p16-c001 — REST/RPC RLS enforcement (tenant isolation).**
  Route every `fdb-reflection` REST CRUD + `/rpc` handler through
  `PgBackend::acquire(rls)` (per-request transaction with the 3 `SET LOCAL`
  GUCs). Give the reflection compiler a non-owner, RLS-subject pool distinct
  from the migration-owner pool. Set `FORCE ROW LEVEL SECURITY` on all
  tenant tables in migrations. Correct the stale doc-comments that claim RLS is
  already enforced.
  - **Gate:** a `DATABASE_URL` integration test seeds two tenants with RLS
    policies and asserts tenant A cannot read or mutate tenant B's rows through
    the real Axum HTTP router — for GET, POST, PATCH, DELETE, and `/rpc`.

- **p16-c002 — Kiln supply-chain trust: signatures + real content hashing.**
  Add a signature field to `FunctionManifest`; make `fke-server` depend on
  `fke-sign-did`/`fke-sign-cosign` and verify at both admin-register and
  invoke ingress; reject unsigned/invalid components. Replace the fake
  `sha256_hex` pseudo-hash in `fke-registry` and `fke-store-fs` with real
  `sha2::Sha256`. Replace cosign's substring "Fulcio" check with real
  certificate-chain + SCT + OIDC-identity binding.
  - **Gate:** an unsigned component and a tampered-bytes component are both
    rejected (403/422) at register and at invoke; a valid signed component runs;
    tests cover all three.

- **p16-c003 — Kiln sandbox + authorization.**
  Apply each component's declared `capabilities` to the Wasmtime linker (stop
  granting full WASI unconditionally); fix `check_capabilities(granted, granted)`
  to compare requested-vs-granted; wire the `flint:host` capability surface
  behind Cedar. Require authentication on `/functions/v1/<name>` and on the
  `/admin/functions` control plane; fail closed when `caller = None`.
  - **Gate:** anonymous invocation is denied; a component requesting an
    ungranted capability is denied at instantiate; admin register without a
    valid token is 401; tests cover each.

- **p16-c004 — Realtime delivery by default.**
  Make the default change-stream source actually emit events: either default to
  the working `listen` (LISTEN/NOTIFY) adapter, or fail loudly (503 + startup
  error) instead of returning an empty stream. Set `FLINT_CHANGE_SOURCE` in
  `docker-compose*.yml` to match the Helm chart.
  - **Gate:** a subscription integration test receives a real change event on
    the default configuration; no silent empty-stream path remains.

### P1 — Correctness & hygiene required before a clean release

- **p16-c005 — Auth hardening.**
  Add JWKS TTL/background refresh (or refetch-on-unknown-kid) so key rotation
  does not require a restart. Make audience validation mandatory (fail closed
  when `FLINT_GATE_AUDIENCE` is unset in production mode).
  - **Gate:** rotating the upstream signing key is picked up without restart in
    a test; a token with a wrong/absent audience is rejected in prod mode.

- **p16-c006 — Config truth & tracker reconcile.**
  Make `ext-flint-hooks` `agui_run` target configurable (remove hardcoded
  `http://localhost:8080`). Correct all stale/misleading doc-comments surfaced
  in the audit. Reconcile the `openspec/changes/` p9–p14 checkboxes with what
  actually shipped so the spec tracker stops under-reporting reality.
  - **Gate:** no hardcoded localhost on a deployable path; `grep` for the stale
    doc claims returns nothing; openspec checkbox state matches code.

- **p16-c007 — 500-line file-size compliance.**
  Split the 17 files over the project's own BLOCKING 500-line limit into
  directory modules (worst: `renderers.rs` 1267, `main.rs` 990, `a2ui.rs` 802).
  - **Gate:** no `crates/**/*.rs` exceeds 500 lines; `cargo check`/`clippy`
    stay green.

### P2 — Operational readiness for real deployment

- **p16-c008 — Production operations.**
  Add production deploy automation (extend `deploy.yml` beyond staging-only).
  Add automated DB backup + PITR (pgBackRest/wal-g or managed-DB wiring);
  stop shipping a "staging only" prod DB. Capture k6 baselines into
  `perf/results/` and wire the regression floor. Enable the `flint_llm` async
  worker by default (or document why sync-only is the supported default).
  - **Gate:** a production deploy runs from CI; a restore-from-backup drill is
    documented and tested; `perf/results/` holds committed baseline numbers.

### P3 — VGV enterprise-standards compliance

- **p16-c009 — Quality gates.**
  Add a coverage gate (`cargo llvm-cov`, ≥90% on changed crates) to CI. Add
  `deny.toml` + `cargo-deny` (license + advisory) alongside the existing
  `cargo audit`. Enforce `#![deny(missing_docs)]` on library crates with
  `# Errors` sections on fallible public fns. De-duplicate the 160 duplicate
  dependency versions. Audit and document every `unsafe` block (19 non-pgrx).
  Review the 47 error-swallow (`.ok()` / `let _ =`) sites on non-test paths.
  - **Gate:** CI enforces coverage + deny; `missing_docs` clean on lib crates;
    `cargo tree --duplicates` materially reduced; every `unsafe` has a
    `// SAFETY:` justification.

---

## Definition of Done for the phase (production-ready 1.0)

1. All four P0 gates pass with committed integration tests.
2. All P1 gates pass.
3. P2 operational gates pass OR are explicitly waived by the operator with a
   documented, tested manual procedure.
4. P3 quality gates pass OR are tracked as accepted debt with owners.
5. `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D
   warnings`, and `cargo test --workspace` all green; pgrx `cargo pgrx test`
   green.
6. A fresh `docs/audits/` re-audit of the P0/P1 paths finds no critical defect.
7. Only then: cut a **new** signed release (`v1.0.1`+ or re-tagged `v1.0.0` with
   corrected notes) that honestly claims production readiness.
