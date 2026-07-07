# Goals — p9-hardening

## Phase Summary

Production hardening of the full Flint stack (Quarry gateway, Kiln runtime,
A2UI registry, React/Flutter SDKs). The codebase is feature-complete as of p8.
p9 closes the gap between "builds and passes tests" and "ships to production
without incident."

Seeded from: `p8-sdk-completeness/reflection.md` → "Recommended Next Phase"

---

## Changes (7 planned)

### P0 — Must ship

- **G1 — p9-c001-docker-compose:**
  `docker-compose.yml` for local development — Postgres 18 with all extensions
  (`pg_net`, `pg_cron`, `pgvector`, pgrx extensions), `fdb-gateway`, `fke-server`,
  optional pgAdmin. A single `docker compose up` must start the entire stack.
  Includes health-check wait logic so gateway doesn't start before DB migrations complete.
  Also produce `docker-compose.prod.yml` as a minimal staging variant (no pgAdmin,
  resource limits set, restart policies).

- **G2 — p9-c002-runbook:**
  `docs/runbook.md` — operational reference covering:
  - **Startup** — `docker compose up`, migration verification, seeding, smoke tests
  - **Common errors** — DB connection refused, migration failed, JWT key expired, Kiln artifact not found, Cedar policy denied
  - **Migration procedure** — apply, verify, rollback steps for each migration file
  - **Rollback** — how to revert to a previous image tag; blue/green checklist
  - **On-call escalation** — severity matrix (P0/P1/P2/P3), escalation paths, recovery SLAs
  - **Security contacts** — who to notify on a suspected breach

- **G3 — p9-c003-rate-limiting:**
  Per-IP rate limiting on `fdb-gateway` REST endpoints using a Tower middleware.
  Default: 100 req/s per IP for REST; 20 req/s for `/graphql`; no limit for `/healthz`.
  Configurable via `FLINT_RATE_LIMIT_REST`, `FLINT_RATE_LIMIT_GRAPHQL` env vars.
  Returns `429 Too Many Requests` with `Retry-After` header on breach.
  Use `tower-governor` crate (or `tower::limit::RateLimit` for simpler key-less limiting).

### P1 — Should ship

- **G4 — p9-c004-observability:**
  Structured observability across the stack:
  - Add `tracing-opentelemetry` to `fdb-gateway` and `fke-server`; emit OTLP spans
  - Expose `GET /metrics` (Prometheus text format) via `metrics` + `metrics-exporter-prometheus` crates
  - Key metrics: request duration histograms per route, error rate counters, active DB connections, Kiln invocation count/duration
  - `observability/grafana-dashboard.json` — importable Grafana dashboard template

- **G5 — p9-c005-security-audit:**
  OWASP Top 10 review of `fdb-gateway` and `fke-server`:
  - Verify no JWT payloads, bearer tokens, or claim values in any `tracing` spans or log output
  - Confirm all user-facing error messages are opaque (no stack traces, no SQL errors)
  - Replace `AllowAllPolicySource` in Kiln with `DbKilnPolicySource` + a real default-deny bootstrap policy
  - Audit all `unwrap()`/`expect()` calls remaining in binary crates; document or eliminate
  - Verify `X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy` response headers on REST routes

- **G6 — p9-c006-performance-audit:**
  Load test `fdb-gateway` REST endpoints:
  - Tool: `k6` or `wrk` script in `perf/`
  - Target: P99 < 100 ms for `GET /a2ui/v1/components`, `GET /openapi.json`; P99 < 200 ms for `POST /graphql`
  - Identify and fix top-3 bottlenecks found
  - Add `cargo bench` benchmarks for `McpCompiler::compile()` and `parse_design_md()` (compile-time hot paths)

### P2 — Ship if capacity allows

- **G7 — p9-c007-staging-deploy:**
  Terraform module or `docker-compose.staging.yml` for a real staging environment:
  - Single-node staging on a small cloud instance (e.g. 4 vCPU / 8 GB RAM)
  - Automated smoke test suite: `scripts/smoke_test.sh` — health checks, component list, MCP tools endpoint, Kiln invoke
  - GitHub Actions `deploy.yml` — on manual trigger or tag push, deploy to staging and run smoke tests

---

## Phase Complete When (MVP gate)

- [ ] `docker compose up` starts the full stack (Postgres + migrations + seed + gateway + Kiln)
- [ ] `docs/runbook.md` covers startup, 5 common errors, migration, and rollback
- [ ] Rate limiting middleware active on fdb-gateway REST routes
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean

---

## Dependencies

### All resolved (p8 + p7b + p6b)
- Docker images: `docker/fdb-gateway/Dockerfile`, `docker/fke-server/Dockerfile` — ✅ p8-c003
- CI pipeline: `.github/workflows/ci.yml` — ✅ p8-c003
- `DbKilnPolicySource` — ✅ p7b-c002
- Full feature stack (Quarry, Kiln, A2UI, SDK) — ✅ p8

### New dependencies
- OQ-P9-1: `tower-governor` vs `tower::limit` — evaluate both before choosing
- OQ-P9-2: `tracing-opentelemetry` OTLP exporter version (0.26 targets OTel 0.27)
- OQ-P9-3: Staging cloud provider and instance type
