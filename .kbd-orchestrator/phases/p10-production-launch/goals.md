# Goals — p10-production-launch

## Phase Summary

Ship Flint Forge to production. Phase 9 closed the gap between "builds and
passes tests" and "runs reliably on a staging host." Phase 10 closes the
remaining gap between "runs on staging" and "accepts real traffic safely."

The three hard requirements for accepting production traffic are TLS termination,
secret rotation out of `.env`, and a live dependency CVE gate. Everything else
in this phase improves operational confidence and supports a clean `v1.0.0`
release announcement.

Seeded from: `p9-hardening/reflection.md` → "Recommended Next Phase"

---

## Changes (6 planned)

### P0 — Must ship (minimum to accept real traffic)

- **G1 — p10-c001-tls-termination:**
  Add TLS/HTTPS to `docker-compose.prod.yml` via Caddy reverse proxy.
  - Add a `caddy` service to `docker-compose.prod.yml` using the official
    `caddy:2-alpine` image.
  - Configure Caddyfile to terminate TLS for `fdb-gateway` (port 443→8080)
    and `fke-server` (port 8443→8090); automatic Let's Encrypt cert provisioning
    via `tls {email}` directive.
  - Remove exposed ports 8080/8090 from `fdb-gateway`/`fke-server` in the
    prod compose — traffic flows only through Caddy.
  - Add `CADDY_TLS_EMAIL` to `.env.example`.
  - Update `docs/runbook.md` with TLS startup and cert-renewal notes.

- **G2 — p10-c002-secrets-management:**
  Move all secrets out of `.env` into a proper store.
  - Add `docker/secrets/` mount pattern to `docker-compose.prod.yml` using
    Docker Swarm secrets (or `secrets:` key in Compose v3.9).
  - Secrets to migrate: `POSTGRES_PASSWORD`, `JWT_SECRET`, `CADDY_TLS_EMAIL`,
    `OTEL_EXPORTER_OTLP_ENDPOINT`.
  - Add a `scripts/rotate_secrets.sh` helper that generates new random values
    and writes them to the correct Docker secret targets.
  - Document the rotation procedure in `docs/runbook.md §10`.
  - Update `.env.example` — mark migrated vars as `# MANAGED VIA DOCKER SECRET`.

- **G3 — p10-c003-cargo-audit-ci:**
  Add `cargo audit` to the CI pipeline; fail on CVSS ≥ 7.0 findings.
  - Add `cargo audit` step to `.github/workflows/ci.yml` after the existing
    clippy + test steps.
  - Add `cargo-audit` to the dev-toolchain setup step (via `cargo install
    cargo-audit --locked`).
  - Add `audit.toml` (`.cargo/audit.toml`) to allowlist any known
    false-positive advisories, documented with justification and expiry date.
  - Gate: workflow fails if any unfixed advisory has CVSS ≥ 7.0.

### P1 — Should ship

- **G4 — p10-c004-alerting-rules:**
  Wire Prometheus alerting rules to Alertmanager.
  - Create `observability/alerts.rules.yml` with 4 rules:
    - `HighErrorRate`: error rate > 1% for 5 minutes
    - `HighP99Latency`: P99 > 500 ms for 5 minutes
    - `ServiceDown`: `up == 0` for 1 minute
    - `HighDbConnections`: pool connections > 8 for 3 minutes
  - Add `alertmanager` service to `docker-compose.prod.yml` with a skeleton
    `observability/alertmanager.yml` (webhook receiver stub; operator fills
    PagerDuty/Slack URLs).
  - Add `prometheus` service to `docker-compose.prod.yml` with the rules file
    mounted.
  - Update Grafana dashboard JSON to import the alerting annotations.

- **G5 — p10-c005-k6-baseline:**
  Replace aspirational performance targets with measured P99s.
  - Run `perf/k6/` scripts against a live staging stack (via GitHub Actions
    job or manual run on staging host).
  - Record P50/P95/P99 and throughput in `docs/performance.md` as the
    official baseline.
  - Add a `perf/k6/regression.js` script that fails if P99 exceeds the
    recorded baseline by > 20%.
  - Add an optional `performance` job to `.github/workflows/ci.yml` (manual
    trigger or `workflow_dispatch`; not a required gate due to staging
    dependency).

### P2 — Ship if capacity allows

- **G6 — p10-c006-changelog-release:**
  Conventional commit changelog + `v1.0.0` release tag.
  - Add `cliff.toml` (git-cliff config) for conventional commit changelog generation.
  - Generate `CHANGELOG.md` from all commits through HEAD.
  - Tag `v1.0.0` on `main`; update `[workspace.package] version` to `1.0.0`.
  - Create GitHub Release via `gh release create` with:
    - Changelog section for this release
    - Docker image digests for `fdb-gateway:1.0.0` and `fke-server:1.0.0`
    - Signed artifacts (cosign) if the signing infra from p6b is wired into CI.

---

## Phase Complete When (MVP gate)

- [ ] `docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d` starts the full stack with TLS
- [ ] Secrets are not in `.env`; `scripts/rotate_secrets.sh` is documented
- [ ] `cargo audit` gate is live in CI; 0 unfixed CVSS ≥ 7.0 advisories
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean

---

## Open Debt Inherited from p9

| Item | Source | Severity | Resolution path |
|---|---|---|---|
| k6 baselines are aspirational | p9-c006 | LOW | G5 (p10-c005) replaces them with measured values |
| `STAGING_SMOKE_TOKEN` is long-lived JWT | p9-c007 | MEDIUM | G2 (p10-c002) introduces proper secret rotation |
| Grafana DB connections panel needs sqlx integration | p9-c004 | LOW | G4 (p10-c004) wires Prometheus; sqlx metrics deferred |
| `version: '3.9'` cosmetic noise in compose files | p9 | COSMETIC | Fix inline during G1 (TLS compose work) |

---

## Dependencies

All resolved (p9 + p8):

- Docker images: `docker/fdb-gateway/Dockerfile`, `docker/fke-server/Dockerfile` — ✅ p8-c003
- CI pipeline: `.github/workflows/ci.yml` — ✅ p8-c003
- Compose base: `docker-compose.yml`, `docker-compose.prod.yml` — ✅ p9-c001
- Observability: `observability/grafana-dashboard.json`, `/metrics` endpoint — ✅ p9-c004
- Staging deploy: `docker-compose.staging.yml`, `smoke_test.sh`, `deploy.yml` — ✅ p9-c007

### New dependencies

- OQ-P10-1: TLS strategy — Caddy (preferred, automatic ACME) vs Nginx (manual cert management)?
  → Caddy chosen (noted in G1 description); revisit only if operator's infra requires Nginx.
- OQ-P10-2: Secrets backend — Docker secrets (Compose), Vault, or cloud SSM?
  → Docker Compose `secrets:` chosen for provider-agnostic portability; Vault deferred.
- OQ-P10-3: Version scheme — `v1.0.0` vs `v0.10.0`?
  → Deferred to G6 assessment; p9 hardening closes the v1 readiness bar.
