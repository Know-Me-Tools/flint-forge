# Goals — p11-api-stability

## Phase Summary

Close the remaining gap between `v0.10.0` (production-hardened, running on staging)
and `v1.0.0` (stable public API contract). The Flint platform's three primary API
surfaces — the A2UI component registry, the Kiln WASM ABI, and the React/Flutter
SDKs — must be frozen and documented before downstream skill authors and SDK
consumers can rely on them. This phase also resolves three operational debt items
inherited from p10: incomplete Docker secrets wiring, aspirational k6 baselines,
and a long-lived staging JWT.

Seeded from: `p10-production-launch/reflection.md` → "Recommended Next Phase"

---

## Changes (6 planned)

### P0 — Must ship (required for v1.0.0 readiness)

- **G1 — p11-c001-a2ui-api-freeze:**
  Document and freeze the A2UI public component registry API:
  - Add `#[non_exhaustive]` to all public enums in `fdb-domain` that form part
    of the A2UI surface (`SlugKind`, `ComponentKind`, `AssemblyHint`, etc.)
  - Audit `fdb-app/src/a2ui/` for any `pub` items that are implementation details;
    mark with `#[doc(hidden)]` or move to `pub(crate)`
  - Write `docs/api/a2ui.md` — public API reference covering: component schema,
    `/a2ui/v1/components` shape, `/a2ui/v1/surfaces/assemble` contract,
    `/a2ui/v1/applications` contract, and versioning policy
  - Add `FLINT_A2UI_API_VERSION=1` to `.env.example`
  - Gate: `cargo clippy --workspace -- -D warnings` clean; `cargo test --workspace` passes

- **G2 — p11-c002-kiln-abi-freeze:**
  Freeze the Kiln WASM guest ABI and document the skill author contract:
  - Audit `crates/fke-domain/wit/` — ensure all WIT interfaces are stable;
    add `@since` annotations to any interface that changed post-p6
  - Write `docs/api/kiln-abi.md` — skill author reference covering:
    `wasi:http/incoming-handler` contract, fuel limit, epoch interruption behaviour,
    Cedar authz decision flow, `ContentId` format, and supported store backends
  - Tag the WIT interfaces with a `stability: stable` annotation in the `.wit` files
  - Add `FLINT_KILN_ABI_VERSION=1` to `.env.example`
  - Gate: `cargo check --workspace` clean; WIT files parseable by `cargo component`

- **G3 — p11-c003-sdk-v1-alignment:**
  Align `@flint/react` and `flint_genui` with the frozen A2UI and Kiln ABIs:
  - Bump `@flint/react` package.json `version` to `1.0.0`
  - Bump `flint_genui` `pubspec.yaml` `version` to `1.0.0`
  - Write `packages/flint-react/CHANGELOG.md` and `packages/flint_genui/CHANGELOG.md`
    covering breaking changes since the initial scaffold
  - Add `MIGRATION.md` at workspace root documenting the p10→p11 API delta for
    downstream consumers
  - Gate: SDK packages build without errors (`npm run build` / `flutter analyze`)

### P1 — Should ship

- **G4 — p11-c004-k6-baselines:**
  Replace aspirational k6 thresholds with measured values from a live staging run:
  - Run `perf/k6/health.js`, `components.js`, and `mcp_tools.js` against staging
  - Record P50/P95/P99 and throughput in `docs/performance.md` baseline table
  - Update thresholds in `perf/k6/regression.js` to `measured_p99 × 1.20`
  - Add a `baseline_date` comment to `regression.js` so the measurement date is
    tracked in source
  - Gate: `regression.js` passes against staging

- **G5 — p11-c005-entrypoint-secrets:**
  Wire Docker secrets fully for app containers without requiring `.env`:
  - Add `docker/fdb-gateway/entrypoint.sh` — reads `/run/secrets/postgres_password`
    and sets `DATABASE_URL` before exec; reads `/run/secrets/jwt_secret` and
    sets `FLINT_JWT_SECRET` before exec
  - Add `docker/fke-server/entrypoint.sh` — same for `postgres_password`
  - Update both Dockerfiles to `COPY --chown=root:root ... entrypoint.sh` and
    `ENTRYPOINT ["/entrypoint.sh"]`
  - Update `docker-compose.prod.yml`: remove `FLINT_JWT_SECRET_FILE` env hint
    (now handled by entrypoint); update `DATABASE_URL` in prod overlay to
    `postgres://flint@db:5432/flint` (passwordless — password injected by
    entrypoint)
  - Gate: `docker compose -f docker-compose.yml -f docker-compose.prod.yml
    config --quiet` passes

### P2 — Ship if capacity allows

- **G6 — p11-c006-staging-token-rotation:**
  Replace the long-lived `STAGING_SMOKE_TOKEN` JWT with a short-lived identity:
  - Add a `scripts/mint_smoke_token.sh` script that calls the fdb-gateway
    auth endpoint with operator credentials and writes a 1-hour JWT to
    `/tmp/smoke_token`
  - Update `.github/workflows/deploy.yml` to mint a fresh token before calling
    `smoke_test.sh`
  - Document the rotation procedure in `docs/runbook.md §11`

---

## Phase Complete When (MVP gate)

- [ ] `#[non_exhaustive]` applied to all public A2UI enums; `docs/api/a2ui.md` written
- [ ] `docs/api/kiln-abi.md` written; WIT interfaces tagged stable
- [ ] `@flint/react` and `flint_genui` at version 1.0.0
- [ ] `perf/k6/regression.js` thresholds replaced with measured values
- [ ] Dockerfile entrypoints wire secrets without `.env`
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean

---

## Open Debt Inherited from p10

| Item | Source | Resolution path |
|---|---|---|
| k6 thresholds aspirational | p10-c005 | G4 (p11-c004) |
| `DATABASE_URL` via `.env` | p10-c002 | G5 (p11-c005) |
| `STAGING_SMOKE_TOKEN` long-lived | p10 | G6 (p11-c006) |
| Grafana DB connections panel | p10-c004 | Deferred — requires future sqlx integration |
| CHANGELOG covers only p3.x | p10-c006 | Resolved by `cliff.toml` for future releases |

---

## Dependencies

All resolved (p10):

- TLS + Caddy — ✅ p10-c001
- Docker secrets pattern — ✅ p10-c002
- `cargo audit` gate — ✅ p10-c003
- Prometheus + Alertmanager — ✅ p10-c004
- k6 scripts — ✅ p10-c005
- `v0.10.0` release — ✅ p10-c006

### New dependencies

- OQ-P11-1: Does `cargo component` support WIT `@since` / `stability` annotations
  in the version of the toolchain in use? Verify before implementing G2.
- OQ-P11-2: What is the correct process for minting a short-lived smoke token
  without a separate IdP? Clarify whether fdb-gateway can issue its own limited-scope
  JWTs or whether an external flint-gate service is required for G6.
