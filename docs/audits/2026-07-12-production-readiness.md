# Production-Readiness Audit — 2026-07-12

> Source evidence for `RUST_ASSESSMENT.md` and the p16-production-remediation
> KBD assessment. Method: three independent code audits (Quarry, Kiln, Anvil+ops)
> against actual source (not docs), plus a VGV enterprise-standards pass. Run
> through the sycophancy-correction skill (score 0.0, no correction needed).
> Build baseline: `cargo check --workspace` + `cargo clippy --workspace
> --all-targets -- -D warnings` both pass (exit 0).

## Verdict

**Not production-ready.** The `v1.0.0` tag is an API/ABI contract freeze, not an
operational milestone. Four P0 security-critical defects sit on live request
paths; operational hardening (prod CD, backups, measured perf) is undone.

## Critical defects (P0)

1. **REST/RPC tenant-isolation bypass.** Every `fdb-reflection` REST CRUD and
   `/rpc` handler runs SQL on the privileged migration-owner pool with no
   per-request RLS context. `require_rls` (`crates/fdb-gateway/src/rls_layer.rs:31`)
   authenticates the JWT but never issues `SET LOCAL ROLE` / `request.jwt.claims`.
   Handlers use `q.fetch_one(&state.pool)` (`crates/fdb-reflection/src/compilers/rest/mod.rs:166`,
   `mutations.rs`, `rpc.rs:90`); pool origin is the migration owner
   (`crates/fdb-gateway/src/main.rs:88` → `crates/fdb-reflection/src/state_manager.rs:189`).
   Only `/graphql` and `/rpc/vector` correctly use `PgBackend::acquire`. → any
   authenticated tenant can read/mutate another tenant's rows. No test proves REST
   isolation.

2. **Kiln executes unsigned WASM.** No signature verification at any ingress;
   `fke-sign-did`/`fke-sign-cosign` are not a dependency of `fke-server`;
   `FunctionManifest` has no signature field (`crates/fke-domain/src/lib.rs:39`).
   Content-addressing uses a fake pseudo-hash labeled `sha256:` with a self-
   admitting `TODO` (`crates/fke-registry/src/lib.rs:108`). cosign "Fulcio"
   validation is a substring match on the issuer DN
   (`crates/fke-sign-cosign/src/lib.rs:168`).

3. **Kiln sandbox + authz not enforced.** `check_capabilities(granted, granted)`
   is a no-op (`crates/fke-runtime/src/lib.rs:212`); the linker grants full WASI
   to every component regardless of declared capabilities; anonymous invoke
   (`caller = None`) bypasses the Cedar `kiln:invoke` gate; `/admin/functions`
   has no auth.

4. **Realtime empty by default.** Default `FabricChangeSource::watch()` returns
   `stream::empty()` (`crates/fdb-realtime/src/lib.rs:116`, OQ-FRF-1). Working
   `listen` adapter exists but is off unless `FLINT_CHANGE_SOURCE=listen`; Helm
   sets it, compose does not.

## High / P1

- JWKS never refreshes (rotation needs restart); audience skipped when
  `FLINT_GATE_AUDIENCE` unset.
- `ext-flint-hooks` `agui_run` hardcodes `http://localhost:8080`
  (`crates/ext-flint-hooks/sql/flint_hooks.sql:156`).
- Stale/false doc-comments: `rest/mod.rs:62` ("CRUD is `todo!()`", false),
  `rest/mod.rs:120` ("RLS enforced by GUC context", false).
- 17 files exceed the project's own 500-line BLOCKING limit (worst:
  `routes/htmx/renderers.rs` 1267, `main.rs` 990, `routes/a2ui.rs` 802).
- openspec p9–p14 `tasks.md` checkboxes near-universally unchecked while work
  shipped — tracker under-reports reality.

## Operational / P2

- No production deploy automation (`.github/workflows/deploy.yml` staging-only).
- No automated DB backup/PITR; prod compose DB self-described "staging only".
- No captured k6 baselines (`perf/results/` empty); regression gate deferred.
- `ext-flint-llm` async worker off by default.

## VGV / P3

- No coverage gate (`cargo llvm-cov`) in CI; no `deny.toml`/`cargo-deny`
  (`cargo audit` present); `missing_docs` unenforced (1 crate); 160 duplicate
  dependency versions; 19 non-pgrx `unsafe` blocks without `// SAFETY:` docs; 47
  error-swallow (`.ok()` / `let _ =`) sites on non-test paths.

## What is solid (sets the remediation precedent)

- JWT verification in `forge-identity`: real JWKS + signature + exp/iss, fail
  closed, no alg-confusion.
- GraphQL query/mutation + introspection merge (the correct RLS `acquire()`
  pattern REST must adopt).
- All five Anvil pgrx extensions — no security shortcuts; `ext-flint-vault`
  (real XChaCha20-Poly1305 AEAD + HKDF + genuine KMS-envelope DEK, never
  plaintext/SQL-selectable) is the strongest crate.
- Wasmtime fuel + epoch limits genuinely enforced.
- TLS overlay (Caddy/ACME), Docker secrets, rate limiting, security headers,
  Prometheus/Alertmanager, Helm chart, 1093-line operator runbook.

## Remediation

Tracked as phase **p16-production-remediation** (9 changes; see
`.kbd-orchestrator/phases/p16-production-remediation/goals.md`). Every P0 fix has
a correct in-tree precedent, so remediation is wiring + tests, not new
subsystems. Serial critical path to an honest v1.0: ~4–6 weeks; P0 alone ~2–3
weeks.
