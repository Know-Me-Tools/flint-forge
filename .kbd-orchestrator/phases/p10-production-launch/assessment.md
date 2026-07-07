# Assessment — p10-production-launch

**Phase:** 10 — Production Launch
**Assessed:** 2026-07-06
**Assessor:** OpenCode / KBD automated assess
**Changes in scope:** 6 (p10-c001 through p10-c006)
**Prior phase:** p9-hardening (7/7 complete)

---

## Summary

The codebase exits p9 in a strong operational baseline: containerised, rate-limited,
observable, security-headers hardened, staging-deployable. Three gaps block real-traffic
acceptance: no TLS termination, secrets in `.env`, and no live CVE gate. A fourth finding
requires an immediate plan-phase decision: `wasmtime 26` carries two CVSS-9.0 critical
advisories that are directly exploitable when untrusted WASM is executed — which is exactly
Flint Kiln's threat model.

---

## Goal-by-Goal Gap Analysis

### G1 — TLS Termination (`p10-c001`) — ❌ NOT STARTED

**What exists:**

- `docker-compose.prod.yml` has resource limits and restart policies but **no Caddy or Nginx service**
- `fdb-gateway` and `fke-server` expose ports `8080` and `8090` directly in `docker-compose.yml` — traffic reaches the services without any TLS layer
- `.env.example` has no `CADDY_TLS_EMAIL` variable

**Gaps:**

| Gap | Severity | Notes |
|---|---|---|
| No reverse proxy service in prod compose | P0 | Raw HTTP on public port = unacceptable for production |
| Ports 8080/8090 exposed in base compose | P0 | Must be removed or overridden to `127.0.0.1` binding in prod overlay |
| `CADDY_TLS_EMAIL` not in `.env.example` | LOW | Required for ACME Let's Encrypt cert provisioning |
| No TLS section in `docs/runbook.md` | LOW | §10 slot reserved from p9-c007; content not yet written |

**Effort estimate:** Medium. Caddy config is simple (Caddyfile is ~10 lines). The tricky part is correctly overriding port bindings in the compose overlay without breaking the base file.

---

### G2 — Secrets Management (`p10-c002`) — ❌ NOT STARTED

**What exists:**

- `.env.example` carries `FLINT_JWT_SECRET=change-me-in-production` as a plain env var — the path of least resistance for a developer copying to `.env`
- No Docker `secrets:` block in any compose file
- No `scripts/rotate_secrets.sh`
- No per-secret documentation in the runbook

**Gaps:**

| Gap | Severity | Notes |
|---|---|---|
| `FLINT_JWT_SECRET` in cleartext `.env.example` | P0 | Operators copy `.env.example` → `.env`; default "change-me" value ships to production regularly |
| `POSTGRES_PASSWORD` in cleartext (`flint:flint` hardcoded in `DATABASE_URL`) | P0 | Trivially guessable default password in compose |
| No Docker secrets mounting pattern | P0 | Services read secrets from env vars, not from `/run/secrets/` |
| No `scripts/rotate_secrets.sh` | P1 | Runbook §9 promises rotation tooling; not yet built |
| No `MANAGED VIA DOCKER SECRET` markers in `.env.example` | LOW | Documentation gap |

**Effort estimate:** Medium. Docker Compose `secrets:` pattern is well-defined; the real work is updating service Dockerfiles or entrypoints to read from `/run/secrets/` rather than env vars, or using `environment: POSTGRES_PASSWORD_FILE=/run/secrets/postgres_password` pattern.

**Note on scope:** The Docker Compose native secrets pattern (`secrets:` top-level key) works with both standalone Compose and Swarm. It mounts secret files at `/run/secrets/<name>`. Services that already read from env vars need an adapter (entrypoint script or environment variable pointing to a file path, depending on the service). PostgreSQL natively supports `POSTGRES_PASSWORD_FILE`.

---

### G3 — `cargo audit` CI Gate (`p10-c003`) — ❌ NOT STARTED + ⚠️ CRITICAL FINDING

**What exists:**

- `cargo audit` v0.22.1 is installed locally
- CI pipeline (`ci.yml`) has **no `cargo audit` step** — the entire advisory gate is absent from CI
- `.cargo/audit.toml` is **absent** — no allowlist exists for known false positives
- `cargo audit` currently reports **26 vulnerabilities** across the dependency tree

**Current advisory landscape:**

| Severity | Count | Primary crate(s) |
|---|---|---|
| Critical (CVSS 9.0) | **2** | `wasmtime 26` |
| High (CVSS 7.5) | **3** | `wasmtime-wasi` (fxhash), `quick-xml 0.37.5` (×2 via `object_store 0.11`) |
| Medium (CVSS 5.6–6.9) | **9** | `wasmtime` suite, `rustls-webpki`, `rsa` |
| Low (CVSS 1.8–3.3) | **5** | `wasmtime` suite, `paste` |
| Unmaintained/no-fix | **7** | Various indirect deps |

**CVSS ≥ 7.0 blockers (the gate threshold):**

| Advisory | Crate | CVSS | Fix | Impact for Flint |
|---|---|---|---|---|
| RUSTSEC-2026-0096 | `wasmtime 26` | **9.0** | Upgrade to ≥36.0.7 or ≥43.0.1 | **HIGH** — memory-safety vuln in WASM JIT; directly in Kiln's trust boundary |
| RUSTSEC-2026-0095 | `wasmtime 26` | **9.0** | Upgrade to ≥36.0.7 or ≥43.0.1 | **HIGH** — same, companion advisory |
| RUSTSEC-2026-0195 | `quick-xml 0.37.5` | 7.5 | Upgrade `object_store` to version using `quick-xml ≥0.41` | LOW — DoS only; in `fke-store-s3` (S3 object store, not on hot path) |
| RUSTSEC-2026-0194 | `quick-xml 0.37.5` | 7.5 | Same as above | LOW — DoS, quadratic runtime, same path |
| RUSTSEC-2026-0149 | `fxhash` (via wasmtime) | 7.5 | Upgrade wasmtime | Transitively fixed by wasmtime upgrade |

**⚠️ Plan-phase decision required — wasmtime upgrade:**

`wasmtime 26` → `wasmtime ≥36.0.7` is a **10-major-version jump**. The Wasmtime project uses calendar versioning (minor = ~monthly release). The API surface for `component-model`, `async`, and `pooling-allocator` features used in `fke-runtime` changes across this range.

Two options for the plan:

1. **Upgrade wasmtime** (`26` → `36.x`): fixes all CVSS ≥7.0 wasmtime advisories. Requires auditing `fke-runtime/src/lib.rs` for breaking API changes. Medium effort; should compile cleanly since the component model stabilised in ~v20. **Recommended.**
2. **Allowlist the two CVSS-9.0 advisories** in `.cargo/audit.toml` with an expiry date and a justification noting that Kiln's WASM execution currently runs only internally-signed artifacts (from the Cosign/DID gate in p6b), not arbitrary untrusted WASM. This is a short-term mitigation only — the allowlist should expire at next sprint.

The `quick-xml` / `object_store` issue is resolved by bumping `object_store` from `0.11` to `0.12+`, which uses `quick-xml ≥0.41`.

**Gaps:**

| Gap | Severity |
|---|---|
| No `cargo audit` step in CI | P0 |
| No `.cargo/audit.toml` | P0 |
| `wasmtime 26` CVSS-9.0 advisories (2) | **P0 BLOCKER** |
| `quick-xml` CVSS-7.5 via `object_store 0.11` | P0 (fixable by bump) |
| 21 medium/low/unmaintained advisories requiring allowlist decisions | P1 |

---

### G4 — Alerting Rules (`p10-c004`) — ❌ NOT STARTED

**What exists:**

- `observability/grafana-dashboard.json` — 4-panel dashboard (p9-c004)
- `GET /metrics` Prometheus endpoint on fdb-gateway (p9-c004)
- No `alerts.rules.yml`
- No Alertmanager service or config
- No Prometheus scrape config or service in any compose file

**Gaps:**

| Gap | Severity | Notes |
|---|---|---|
| No `observability/alerts.rules.yml` | P1 | 4 alert rules need authoring |
| No Alertmanager service in prod compose | P1 | `alertmanager` service + `observability/alertmanager.yml` |
| No Prometheus scrape service in prod compose | P1 | Prometheus needs to be deployed to consume `/metrics` |
| Grafana dashboard has no alert annotations | LOW | Panel 3 (error rate) should import alerting thresholds |

**Note:** The `/metrics` endpoint exists and is correct Prometheus text format. Prometheus and Alertmanager services need to be added to `docker-compose.prod.yml`; the scrape target is already known (`fdb-gateway:8080/metrics`, `fke-server:8090/metrics`).

---

### G5 — k6 Performance Baseline (`p10-c005`) — ⚠️ PARTIAL

**What exists:**

- `perf/k6/health.js`, `components.js`, `mcp_tools.js` — three k6 scripts
- `perf/k6/README.md` — usage docs
- `docs/performance.md` — aspirational targets (not measured values)

**Gaps:**

| Gap | Severity | Notes |
|---|---|---|
| No measured P50/P95/P99 values in `docs/performance.md` | P1 | Requires live staging stack |
| No `perf/k6/regression.js` gate script | P1 | CI-runnable regression check |
| No `performance` job in `ci.yml` | P1 | Even as a manual-trigger `workflow_dispatch` job |

**Note:** The k6 scripts themselves are complete and correct. The gap is purely operational — they need to be run against a live staging stack and the measured values committed. The `regression.js` script is a new file that fails if P99 exceeds baseline + 20%.

**Blocker:** This goal requires a running staging stack with valid secrets and TLS (goals G1/G2). It should be sequenced after G1 and G2 in the plan.

---

### G6 — CHANGELOG + Release (`p10-c006`) — ❌ NOT STARTED

**What exists:**

- No `CHANGELOG.md`
- No `cliff.toml`
- No git tags (zero tags in the repository)
- `[workspace.package] version = "0.1.0"` in `Cargo.toml`

**Version scheme decision (OQ-P10-3):**

The `0.1.0` workspace version is honest — it reflects that this is a first production-grade build, not a v1.0.0 "stable API" promise. Two options:

1. **Tag `v0.10.0`** — aligns with phase 10; signals pre-stable but production-grade. No version bump in `Cargo.toml`.
2. **Tag `v1.0.0`** — claims API stability. Requires bumping `Cargo.toml` and accepting semver obligations. Premature given the A2UI, Kiln, and SDK APIs are still evolving.

**Recommendation: `v0.10.0`.** This is a meaningful milestone tag without the API-stability commitment of `v1.0.0`.

**Gaps:**

| Gap | Severity | Notes |
|---|---|---|
| No `cliff.toml` | P2 | git-cliff conventional commit changelog config |
| No `CHANGELOG.md` | P2 | Generated from commit history |
| No version tag | P2 | `v0.10.0` recommended over `v1.0.0` |
| `[workspace.package] version` is `0.1.0` | P2 | Bump to `0.10.0` |
| Docker image digests not recorded | P2 | GitHub Release artifact table |

---

## Open Questions — Assessment Findings

| OQ | Question | Assessment finding |
|---|---|---|
| OQ-P10-1 | Caddy vs Nginx | **Caddy confirmed.** Caddyfile is ~10 lines vs Nginx config boilerplate. Automatic ACME avoids cert management. The only counter-case (Nginx) is if an operator's infrastructure already mandates it — document the swap as a runbook variant. |
| OQ-P10-2 | Docker secrets vs Vault vs cloud SSM | **Docker Compose `secrets:` confirmed** for portability. The `secrets:` key works identically in standalone Compose and Swarm. Cloud SSM migration is a future phase concern — out of scope for p10. |
| OQ-P10-3 | Version scheme `v1.0.0` vs `v0.10.0` | **`v0.10.0` recommended.** Avoids premature API stability promise. Tag `v1.0.0` when the A2UI, Kiln, and SDK public APIs are stable. |
| NEW: OQ-P10-4 | wasmtime upgrade scope | **Upgrade to `wasmtime 36.x` recommended** over allowlisting. Two CVSS-9.0 memory-safety CVEs in the WASM JIT are not acceptable in a production Kiln that will eventually execute user-supplied WASM. Scope: `fke-runtime/src/lib.rs` API migration (ProxyPre, PoolingAllocatorConfig, epoch handling). |

---

## Inherited Debt — Status at Phase Start

| Item | Inherited from | Status |
|---|---|---|
| k6 baselines aspirational | p9-c006 | Addressed by G5 |
| `STAGING_SMOKE_TOKEN` long-lived JWT | p9-c007 | Addressed by G2 (rotation script) |
| Grafana DB connections panel (sqlx integration missing) | p9-c004 | Persists — deferred past p10 |
| `version: '3.9'` cosmetic noise in compose files | p9 | Fix inline during G1 work |
| `wasmtime 26` CVEs (not flagged in p9, no audit step existed) | Pre-p9 | NEW P0 BLOCKER — must fix in G3 |

---

## Priority Stack for Planning

Based on dependencies and blockers:

```
P0 — Must ship (MVP gate):
  1. p10-c003-cargo-audit-ci     — gate setup + wasmtime upgrade + object_store bump
  2. p10-c001-tls-termination    — Caddy + prod port isolation
  3. p10-c002-secrets-management — Docker secrets + rotate script

P1 — Should ship (in order):
  4. p10-c004-alerting-rules     — after compose changes from G1/G2 stabilise
  5. p10-c005-k6-baseline        — requires G1+G2 staging stack to be live

P2 — Ship if capacity allows:
  6. p10-c006-changelog-release  — independent of all others; can run any time
```

**Dependency note:** G5 (k6) depends on G1+G2 being deployed to staging. G3 (cargo audit) should run first to surface all blockers before other code changes are made. G6 (release) is fully independent and can run in parallel with any other goal.

---

## MVP Gate — Current Status

| Gate condition | Current state | Gap |
|---|---|---|
| `docker compose -f ... -f docker-compose.prod.yml up -d` starts with TLS | ❌ No TLS service | G1 |
| Secrets not in `.env` | ❌ `FLINT_JWT_SECRET` in `.env.example` | G2 |
| `cargo audit` gate live; 0 unfixed CVSS ≥ 7.0 | ❌ No CI step; 5 CVSS ≥ 7.0 (2 critical) | G3 |
| `cargo test --workspace` passes | ✅ 457 tests | — |
| `cargo clippy --workspace -- -D warnings` clean | ✅ Clean | — |

**Two of five gate conditions already pass.** Three require p10 changes.

---

*Assessment complete. Proceed to `/kbd-plan p10-production-launch`.*
