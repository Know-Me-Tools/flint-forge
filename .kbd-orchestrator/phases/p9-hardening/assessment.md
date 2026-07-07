# Assessment — p9-hardening

**Phase:** p9-hardening
**Assessed:** 2026-07-04
**Previous phase:** p8-sdk-completeness (7/7 done; clippy clean; 451 tests passing)

---

## Codebase Inventory

### What exists

| Artifact | State |
|---|---|
| `docker/fdb-gateway/Dockerfile` | ✅ Present (p8-c003) — multi-stage cargo-chef |
| `docker/fke-server/Dockerfile` | ✅ Present (p8-c003) — multi-stage cargo-chef |
| `.github/workflows/ci.yml` | ✅ Present (p8-c003) |
| `.github/workflows/docker.yml` | ✅ Present (p8-c003) |
| `images/postgres18/Dockerfile` | ✅ Present (p0-c002) |
| `docker-compose.yml` | ❌ Missing |
| `docs/runbook.md` | ❌ Missing |
| Rate-limiting middleware | ❌ Missing |
| OTLP / Prometheus observability | ❌ Missing |
| `perf/` load test scripts | ❌ Missing |
| `cargo bench` benchmarks | ❌ Missing |
| Staging deploy scripts | ❌ Missing |

### Key observations

- `tower` is in `[dev-dependencies]` of `fdb-gateway` but **not** in the main `[dependencies]` — rate limiting middleware would need it as a regular dep
- All middleware in `main.rs` is `axum::middleware::from_fn` (Axum-native) — no Tower service layers yet
- `tracing` is used throughout but only for `warn!`/`info!`/`error!` macros; no structured spans with `#[tracing::instrument]` or `Span::current()`
- `AllowAllPolicySource` is still present in `fke-server/src/kiln_policy.rs` (p7b-c002 flagged as debt)
- `main.rs` `unwrap()` calls exist on line 499 (`unwrap_or(1024)`) and 518 (`unwrap_or_default()`) — both are `.unwrap_or()` not `.unwrap()`, so they are safe
- Postgres 18 image exists in `images/postgres18/`; `docker/postgres/Dockerfile` also exists

---

## Gap Analysis by Goal

### G1 — Docker Compose (P0)

**Current state:** Individual Dockerfiles for `fdb-gateway` and `fke-server` exist. No `docker-compose.yml` at workspace root.

**Gap:**
- No `docker-compose.yml` for local development
- No `docker-compose.prod.yml` for staging
- Postgres 18 image exists at `images/postgres18/Dockerfile` — needs to be referenced
- Extensions (`pg_net`, `pg_cron`, `pgvector`, pgrx extensions) must be present in the DB container

**Work required:**
- Create `docker-compose.yml` with services: `db` (Postgres 18 from `images/postgres18/`), `fdb-gateway`, `fke-server`
- Wire `depends_on` with health-check conditions so services wait for DB migrations
- Mount `migrations/` into the DB container or run them via gateway startup (already done via `sqlx::migrate!`)
- Create `docker-compose.prod.yml` as a prod-safe variant (resource limits, restart policies, no pgAdmin)
- Add `.env.example` with all required env vars

**Effort:** Low-medium — YAML composition, no new code.

---

### G2 — Runbook (P0)

**Current state:** No `docs/runbook.md`. `docs/` contains research files and HTML builds.

**Gap:** Entirely missing. No operator documentation.

**Work required:**
- Create `docs/runbook.md` covering:
  - Startup procedure (docker compose up, migration check, seed verification, smoke test)
  - 5+ common errors with diagnosis and remediation steps
  - Migration procedure (apply, verify, rollback)
  - Rollback checklist (image tag, DB revert)
  - On-call severity matrix and escalation

**Effort:** Low — pure documentation, no code changes.

---

### G3 — Rate Limiting (P0)

**Current state:** No rate-limiting middleware in `fdb-gateway`. `tower` is a dev-dep only.

**Gap:**
- `tower` must be added to `[dependencies]` (or `tower-governor` crate added)
- Rate-limit middleware must be applied to the `app` router before serving
- `FLINT_RATE_LIMIT_REST` and `FLINT_RATE_LIMIT_GRAPHQL` env vars needed

**Design options:**
1. **`tower::limit::ConcurrencyLimit`** — limits concurrent requests, not rate. Simple but coarse.
2. **`tower-governor`** crate — token-bucket per IP, proper HTTP 429 + Retry-After. More correct.
3. **`axum::middleware::from_fn` custom** — manual leaky-bucket via `DashMap<IpAddr, AtomicU64>`. Flexible.

Recommendation: `tower-governor` for correctness; fall back to simple `ConcurrencyLimit` if crate brings too many deps.

**Effort:** Medium — new dep, middleware wiring, env config, tests.

---

### G4 — Observability (P1)

**Current state:** `tracing` macros (`info!`, `warn!`, `error!`) used throughout but no structured spans, no metrics endpoint, no OTLP export.

**Gap:**
- No `#[tracing::instrument]` on request handlers
- No Prometheus `/metrics` endpoint
- No OTLP export configured
- No Grafana dashboard

**Work required:**
- Add `tracing-opentelemetry`, `opentelemetry`, `opentelemetry_otlp` to workspace deps
- Add `metrics`, `metrics-exporter-prometheus`, `axum-prometheus` (or manual) to `fdb-gateway`
- Instrument key handlers with `#[tracing::instrument]`
- Expose `GET /metrics` route
- Create `observability/grafana-dashboard.json`

**Effort:** High — multiple new deps, non-trivial OTel setup, Grafana JSON authoring.

**Risk:** `tracing-opentelemetry 0.26` + `opentelemetry 0.27` have a shifting API. Pin versions carefully.

---

### G5 — Security Audit (P1)

**Current state:** Several items need review:

| Item | Status |
|---|---|
| JWT payloads in logs | CLAUDE.md Rule #1 enforced — no log calls with bearer/claims |
| Opaque error messages | REST handlers return `{"error": "internal server error"}` — compliant |
| `AllowAllPolicySource` in Kiln | ⚠️ Still present; `main.rs` uses `DbKilnPolicySource` but stub remains |
| `unwrap()` / `expect()` in binaries | Only `.unwrap_or()` / `.unwrap_or_else()` variants — safe |
| Response security headers | ❌ Not set — no `X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy` |
| Input validation on MCP tool names | ⚠️ `tools/call` passes tool name directly; malformed names return errors but not validated upfront |

**Work required:**
- Add Tower `SetResponseHeaderLayer` for security headers on all REST routes
- Delete `AllowAllPolicySource` struct (or rename `_test_only` to make intent explicit)
- Add `tracing::Span::current().record("error", ...)` pattern — never log the span context containing bearer
- Document OWASP audit results in `docs/security-audit.md`
- Write test: security headers present on all responses

**Effort:** Medium — mostly headers middleware + documentation + cleanup.

---

### G6 — Performance Audit (P1)

**Current state:** No load tests, no benchmarks, no `perf/` directory.

**Gap:**
- No `k6` or `wrk` scripts
- No `cargo bench` benchmarks for `McpCompiler::compile()` or `parse_design_md()`
- No baseline P99 measurements

**Work required:**
- Create `perf/k6/` with scripts for `GET /a2ui/v1/components`, `GET /openapi.json`, `POST /graphql`
- Create `benches/` in `fdb-reflection` for `McpCompiler::compile()` with `criterion`
- Create `benches/` in `fdb-app` for `parse_design_md()` with `criterion`
- Add `criterion = "0.5"` to workspace dev-deps
- Run benchmarks, identify bottlenecks, document in `docs/performance.md`

**Effort:** High — requires a running server for load tests, new benchmark infrastructure. **Blocked on live environment** for k6 tests.

---

### G7 — Staging Deploy (P2)

**Current state:** `.github/workflows/docker.yml` builds and pushes images. No staging compose or Terraform. No smoke test script.

**Gap:**
- No `docker-compose.staging.yml`
- No `scripts/smoke_test.sh`
- No `deploy.yml` GitHub Actions workflow
- No Terraform (optional — compose is simpler)

**Work required:**
- Create `docker-compose.staging.yml` — production variant with real resource limits
- Create `scripts/smoke_test.sh` — curl health checks, component list, MCP tools endpoint
- Create `.github/workflows/deploy.yml` — manual trigger, SSH to staging host, compose pull + up, run smoke tests

**Effort:** Medium — shell scripting + GitHub Actions YAML + SSH configuration.

---

## Dependency Map

```
G1 (Docker Compose)    — independent; uses existing Dockerfiles from p8
G2 (Runbook)           — independent; pure docs
G3 (Rate limiting)     — independent; tower-governor dep needed
G4 (Observability)     — independent; heavy dep footprint
G5 (Security audit)    — independent; partially Rust, partially docs
G6 (Performance audit) — partially blocked on live server for k6; cargo bench is independent
G7 (Staging deploy)    — depends on G1 (docker-compose.staging.yml extends G1)
```

Optimal order: **G1 + G2 + G3 in parallel** (P0, no external deps) → **G5 + partial G6 (bench only) in parallel** → G4, G7.

---

## Risk Register

| Risk | Likelihood | Severity | Mitigation |
|---|---|---|---|
| `tracing-opentelemetry` API breaking changes | HIGH | MEDIUM | Pin exact version; use `tokio-otel-metrics` pattern rather than full OTel SDK |
| `tower-governor` IP extraction fails behind reverse proxy | MEDIUM | MEDIUM | Use `X-Forwarded-For` header extraction; document proxy setup requirement |
| k6 load tests require live DB and can't run in CI | MEDIUM | LOW | Gate k6 tests behind `--features load-test` or manual CI trigger |
| `docker-compose.yml` Postgres 18 image takes >5 min to build | LOW | LOW | Push pre-built `images/postgres18` to ghcr.io as a CI step |

---

## Assessment Summary

| Goal | Gap Size | Effort | Blocking? |
|---|---|---|---|
| G1 Docker Compose | Medium — YAML composition using existing Dockerfiles | Low-Med | G7 depends on it |
| G2 Runbook | Large — pure docs, no code | Low | No |
| G3 Rate limiting | Small-Med — 1 new dep + middleware wiring | Medium | No |
| G4 Observability | Large — multiple new deps, OTel setup | High | No |
| G5 Security audit | Medium — headers middleware + cleanup + docs | Medium | No |
| G6 Perf audit | Medium — cargo bench independent; k6 blocked on live env | Medium | No (bench part) |
| G7 Staging deploy | Medium — shell scripts + GH Actions YAML | Medium | After G1 |

**No hard external blockers for P0 changes.** G6 k6 tests need a live server; `cargo bench` portion is fully independent.

**Handoff to plan:** Start with G1 (Docker Compose) + G2 (Runbook) + G3 (Rate limiting) in parallel — all three are P0, independent, and well-defined. G5 (Security audit + headers) can run alongside. G4 (Observability) and G6/G7 follow.
