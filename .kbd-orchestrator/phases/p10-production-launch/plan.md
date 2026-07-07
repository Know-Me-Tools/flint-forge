# Plan — p10-production-launch

**Phase:** 10 — Production Launch
**Authored:** 2026-07-06
**Change backend:** OpenSpec
**Changes:** 6 ordered
**Seeded from:** `assessment.md`

---

## Ordering Rationale

The dependency graph is:

```
p10-c003 (CVE fix + audit gate)
    └── p10-c001 (TLS compose)
            └── p10-c002 (secrets compose)
                    └── p10-c004 (Prometheus + Alertmanager in compose)
                            └── p10-c005 (k6 regression — needs live stack)

p10-c006 (CHANGELOG + tag) — fully independent; no deps
```

`p10-c003` runs first because:
1. The wasmtime upgrade changes `fke-runtime/src/lib.rs` — any other
   Rust change should apply on top of a clean compile.
2. The `cargo audit` step will block CI as soon as it's added unless
   the CVSS ≥ 7.0 advisories are already resolved.
3. All other P0/P1 changes are infrastructure-only (compose, scripts) —
   they can be layered cleanly onto a green Rust workspace.

`p10-c006` (release) can run in parallel with any of the above but is
most meaningful after the P0 changes are complete. Execute last.

---

## Change List

### 1. `p10-c003-cargo-audit-ci` — P0 — Execute first

**Goal:** Resolve all CVSS ≥ 7.0 advisories; add `cargo audit` CI gate.

**Scope:**
- `Cargo.toml` workspace deps: `wasmtime 26 → 46`, `wasmtime-wasi 26 → 46`,
  `wasmtime-wasi-http 26 → 46`, `object_store 0.11 → 0.14`
- `crates/fke-runtime/src/lib.rs` — API migration for wasmtime 46
- `crates/fke-store-s3/src/lib.rs` — API migration for object_store 0.14 (if any)
- `.cargo/audit.toml` — allowlist with justifications for no-fix advisories
- `.github/workflows/ci.yml` — add `Security audit` step

**Expected effort:** Medium. The component-model, async, epoch, and fuel
APIs are stable across wasmtime 26→46. The main risk is `wasmtime-wasi` and
`wasmtime-wasi-http` crate API changes. Verify `WasiView`, `WasiHttpView`,
`ProxyPre`, `WasiHttpCtx`, and `ResourceTable` import paths in `fke-runtime`.

**Gate:** `cargo audit` reports 0 CVSS ≥ 7.0 unfixed; CI step green.

---

### 2. `p10-c001-tls-termination` — P0 — Execute second

**Goal:** TLS-terminate all production traffic via Caddy.

**Scope:**
- Remove `version: '3.9'` from all compose files (cosmetic debt cleanup)
- `docker/caddy/Caddyfile` — new file
- `docker-compose.prod.yml` — add `caddy` service; remove public port bindings
  from `fdb-gateway` and `fke-server`
- `.env.example` — add `FLINT_DOMAIN`, `CADDY_TLS_EMAIL`
- `docs/runbook.md` — add §10 (TLS startup, cert renewal, troubleshooting)

**Expected effort:** Small. Caddyfile is ~10 lines; compose changes are additive.
The tricky part is the `ports: !reset []` override — verify Docker Compose v2
syntax for removing inherited port bindings.

**Gate:** `docker compose -f docker-compose.yml -f docker-compose.prod.yml config --quiet` passes.

---

### 3. `p10-c002-secrets-management` — P0 — Execute third

**Goal:** Move secrets out of `.env` into Docker Compose `secrets:` file mounts.

**Scope:**
- Add `secrets/` to `.gitignore`
- `scripts/rotate_secrets.sh` — generate `secrets/*.txt` files
- `docker-compose.prod.yml` — add `secrets:` top-level block; update service
  definitions to mount secrets; update `DATABASE_URL` to passwordless form
- `.env.example` — annotate migrated vars
- `scripts/README.md` — document `rotate_secrets.sh`
- `docs/runbook.md §10` — rotation procedure

**Expected effort:** Small–medium. The Docker secrets pattern is well-defined;
the main complexity is wiring `POSTGRES_PASSWORD_FILE` for Postgres and
`FLINT_JWT_SECRET_FILE` for the gateway (requires checking how `fdb-auth`
reads the secret — may need a small entrypoint change or env var file).

**Gate:** `docker compose -f docker-compose.yml -f docker-compose.prod.yml config --quiet` passes.

---

### 4. `p10-c004-alerting-rules` — P1 — Execute fourth

**Goal:** Prometheus scraping + 4 alert rules + Alertmanager in prod stack.

**Scope:**
- `observability/prometheus.yml` — new file
- `observability/alerts.rules.yml` — new file, 4 rules
- `observability/alertmanager.yml` — new file, webhook stub
- `docker-compose.prod.yml` — add `prometheus` + `alertmanager` services + `prometheus_data` volume
- `.env.example` — add `ALERTMANAGER_WEBHOOK_URL`

**Expected effort:** Small. Pure infrastructure. No Rust code changes.

**Gate:** `docker compose ... config --quiet` passes; rules file is valid YAML.

---

### 5. `p10-c005-k6-baseline` — P1 — Execute fifth

**Goal:** Regression gate script + CI job for performance.

**Scope:**
- `perf/k6/regression.js` — new file with threshold-based pass/fail
- `.github/workflows/ci.yml` — add `performance` job (`workflow_dispatch` only)
- `docs/performance.md` — update with measured or placeholder baselines
- `perf/k6/README.md` — document `regression.js`

**Expected effort:** Small. The regression script is ~40 lines. Measured
baseline values require a live staging stack; if unavailable, ship with
conservative placeholder thresholds and a TODO comment.

**Gate:** `perf/k6/regression.js` parses without errors; CI job definition validates.

---

### 6. `p10-c006-changelog-release` — P2 — Execute last

**Goal:** First production release: `v0.10.0` tag + CHANGELOG + GitHub Release.

**Scope:**
- `cliff.toml` — git-cliff config
- `CHANGELOG.md` — generated from commit history
- `Cargo.toml` — bump `version = "0.10.0"`
- Git tag `v0.10.0` + GitHub Release with image digests

**Expected effort:** Small. Mechanical steps.

**Gate:** `git tag` shows `v0.10.0`; GitHub Release created with CHANGELOG content.

---

## Build / Quality Gates (apply after every change)

```bash
cargo check --workspace          # fast loop gate
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo audit                      # after p10-c003 only; must be 0 CVSS≥7.0
docker compose -f docker-compose.yml -f docker-compose.prod.yml config --quiet
```

---

## MVP Gate Checklist

- [ ] `docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d` starts full stack with TLS
- [ ] Secrets not in `.env`; `scripts/rotate_secrets.sh` documented
- [ ] `cargo audit` gate live in CI; 0 unfixed CVSS ≥ 7.0 advisories
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean
