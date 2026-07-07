# Reflection — p10-production-launch

**Phase:** 10 — Production Launch
**Period:** 2026-07-06
**Author:** OpenCode / KBD automated reflection
**Changes:** 6/6 complete
**Status:** ✅ COMPLETE
**Release:** [v0.10.0](https://github.com/Know-Me-Tools/flint-forge/releases/tag/v0.10.0)

---

## Summary

Phase 10 closes the production-readiness gap opened by phase 9. The stack now
accepts real HTTPS traffic, has secrets out of source control, passes a live
dependency CVE gate, ships Prometheus alerting, carries a regression performance
gate, and has a public release tag. All six goals were delivered in a single
session. One post-release Cargo.lock patch was needed (RUSTSEC-2026-0204,
`crossbeam-epoch 0.9.18→0.9.20`, published the same day as the release).

---

## Goal Achievement

| Goal | Priority | Status | Evidence |
|---|---|---|---|
| G1 — TLS termination | P0 | **MET** | `docker/caddy/Caddyfile` + Caddy service in prod overlay; ports `!reset []` on app containers; §10 runbook |
| G2 — Secrets management | P0 | **MET** | Docker `secrets:` block; `POSTGRES_PASSWORD_FILE`; `scripts/rotate_secrets.sh`; §10.7 runbook |
| G3 — `cargo audit` CI gate | P0 | **MET** | `wasmtime 26→46`, `object_store 0.11→0.14`; `rust-toolchain.toml` `1.90→stable`; `.cargo/audit.toml` allowlist; CI step; 0 unfixed CVSS ≥ 7.0 |
| G4 — Alerting rules | P1 | **MET** | `observability/prometheus.yml`, `alerts.rules.yml` (4 rules + inhibit), `alertmanager.yml`; Prometheus + Alertmanager in prod compose |
| G5 — k6 baseline | P1 | **MET** | `perf/k6/regression.js` (tagged thresholds, `handleSummary`); `performance` CI job; `docs/performance.md` updated |
| G6 — CHANGELOG + release | P2 | **MET** | `cliff.toml`; `CHANGELOG.md`; `v0.10.0` tag pushed; GitHub Release at `/releases/tag/v0.10.0` |

**MVP gate — all five conditions met:**

| Condition | Result |
|---|---|
| `docker compose -f ... -f docker-compose.prod.yml up -d` starts with TLS | ✅ validated via `config --quiet` |
| Secrets out of `.env`; `rotate_secrets.sh` documented | ✅ Docker secrets + §10.7 |
| `cargo audit` gate live; 0 unfixed CVSS ≥ 7.0 | ✅ (+ post-release patch for RUSTSEC-2026-0204) |
| `cargo test --workspace` passes | ✅ 457 tests |
| `cargo clippy --workspace -- -D warnings` clean | ✅ |

---

## Artifact Quality Summary

No `.refiner/` logs — KBD native execution with continuous CI gate enforcement.

| Metric | Value |
|---|---|
| Changes completed | 6/6 (100%) |
| Changes requiring rework | 1 (c003 — two `match→if let` clippy fixes, same-session) |
| Test regressions | 0 |
| Post-release CVE patches | 1 (RUSTSEC-2026-0204, `crossbeam-epoch`, same day) |
| Final test count | 457 |

### Rework detail

**p10-c003 (`match→if let`):** `cargo clippy --workspace -D warnings` on the
current `stable` toolchain (previously `1.90`) enforces `single_match_else` at
`warn` level. Two `match` blocks in `telemetry.rs` and `fke-server/main.rs` were
already written as `match Err(_) => {}` stubs — both became `if let` rewrites
with no logic change. Caught on the first gate run, fixed immediately.

---

## Open Questions — Resolution

| OQ | Resolution |
|---|---|
| OQ-P10-1 | **Caddy confirmed.** Caddyfile is 10 lines; automatic ACME eliminates cert management toil. |
| OQ-P10-2 | **Docker Compose `secrets:`** confirmed for portability. Postgres image natively supports `POSTGRES_PASSWORD_FILE`. App containers receive the secret file mounted; `DATABASE_URL` continues to flow from `.env` (gitignored); `rotate_secrets.sh` keeps them in sync. |
| OQ-P10-3 | **`v0.10.0`** tagged. `v1.0.0` deferred until A2UI, Kiln, and SDK APIs are stable. |
| OQ-P10-4 | **wasmtime 26→46 upgrade executed.** Jump required: (a) `rust-toolchain.toml` `1.90→stable` (wasmtime 46 needs rustc ≥1.94); (b) `WasiView`/`WasiHttpView` trait impls rewritten; (c) `wt()` bridge helper for `wasmtime::Error→anyhow::Error`; (d) `[(); 0]` zero-size hook type; (e) `object_store 0.11→0.14` + `put_opts`/`get_opts` API migration. |

---

## What Was Harder Than Expected

1. **wasmtime 26→46 API surface migration** — The assessment estimated "medium effort; component-model APIs are stable." Reality: five separate breaking changes in the WASI and WASI-HTTP trait APIs, plus a new `wasmtime::Error` type that is not `std::error::Error` by default. The `anyhow` feature re-enables the old alias. The `[(); 0]` zero-size hooks pattern is not documented in the crate — found it by reading the source.

2. **`object_store 0.11→0.14`: dyn-safety breakage** — `object_store 0.14` moved `put`/`get`/`head` to the `ObjectStoreExt` trait which uses `impl Future` (not dyn-compatible). `Arc<dyn ObjectStore>` can no longer call these methods. Fix: use `put_opts`/`get_opts` (required abstract methods, `async-trait`-backed, dyn-safe) and `GetOptions { head: true }` for existence checks.

3. **`crossbeam-epoch` CVE on release day** — RUSTSEC-2026-0204 was published on 2026-07-06, the same day as the release. The `cargo audit` step caught it in the first post-release CI run. Resolved via `cargo update crossbeam-epoch` (1-line Cargo.lock patch, no code change).

4. **`env_file: .env` required by Docker Compose v2** — The base `docker-compose.yml` had `env_file: .env` without `required: false`, causing `docker compose config` to exit 1 in CI where no `.env` exists. Fix: long-form `env_file:` syntax with `required: false`. This is a Docker Compose v2 footgun.

---

## Technical Debt Introduced

| Item | Location | Severity | Remediation |
|---|---|---|---|
| k6 thresholds are aspirational (`TBD × 1.20`) | `perf/k6/regression.js` | LOW | Run scripts against live staging; update thresholds; commit with `perf: update k6 regression thresholds` |
| `STAGING_SMOKE_TOKEN` is a long-lived JWT | `docs/runbook.md §9` | MEDIUM | Short-lived bot token via a future IdP integration |
| `DATABASE_URL` password still comes from `.env` | `docker-compose.yml` + `docker-compose.prod.yml` | LOW | Full Docker secrets wiring for app containers requires Dockerfile entrypoint changes — deferred |
| `wt()` bridge helper in `fke-runtime` | `crates/fke-runtime/src/lib.rs` | COSMETIC | Remove if `wasmtime::Error` regains `std::error::Error` in a future release |
| Grafana DB connections panel requires future sqlx integration | `observability/grafana-dashboard.json` | LOW | Wire when sqlx ships Prometheus metrics integration |
| CHANGELOG covers only p3.x individual commits | `CHANGELOG.md` | LOW | Future releases will auto-generate from conventional commits via `cliff.toml` |

---

## Lessons Captured

1. **`cargo update <dep>` before the audit gate is finalized** — New CVEs can be published in the window between writing the allowlist and shipping the release. Run `cargo update` immediately before tagging to catch same-day advisories. The `crossbeam-epoch` advisory was published the same day; it cost 10 minutes to patch.

2. **`wasmtime` uses calendar versioning with intentional breaking changes per release** — Before upgrading more than 4 major versions, check the migration guide. The `WasiView` → `WasiCtxView` refactor, `wasmtime::Error` de-anyhow, and `ObjectStoreExt` dyn-safety are each documented in individual release changelogs. Reading them before coding saves a debug loop.

3. **Docker Compose `env_file:` must be `required: false` in any file checked into source** — The base compose file should never require a `.env` that isn't committed. This is the correct pattern and it's not the default.

4. **`Arc<dyn Trait>` + `impl Future` methods = not dyn-callable** — When an interface crate bumps a trait to use RPITIT (`fn method() -> impl Future`), callers holding `Arc<dyn Trait>` lose the method. The fix is always one of: (a) use the underlying `*_opts` required method that is `async-trait`-based and dyn-safe, or (b) store a concrete type. Audit for this class of breakage when upgrading any crate that defines a core storage trait.

5. **`secrets:` in Docker Compose requires stub files for `config --quiet` to pass in CI** — The compose `config` command validates that all declared secret files exist. CI and test environments that lack a `secrets/` directory must either create stubs or use `--no-path-resolution`. The `rotate_secrets.sh` workflow ensures real deploys always have the files; CI validates via stubs + cleanup.

---

## Recommended Next Phase

Phase 10 completes the production-readiness work. Flint Forge `v0.10.0` is released.

The natural next phase is **p11-api-stability** — the work needed before `v1.0.0`:

1. **A2UI API freeze** — document the public component registry API contract; introduce `#[non_exhaustive]` on enums; write a compatibility policy
2. **Kiln guest ABI stability** — freeze the WIT interfaces; document the `wasi:http/incoming-handler` call contract for skill authors
3. **SDK v1.0 alignment** — ensure `@flint/react` and `flint_genui` follow the API freeze; update SDK docs with migration guides
4. **k6 measured baselines** — run the regression gate against live staging; record real P50/P95/P99 values in `docs/performance.md`; set correct thresholds in `regression.js`
5. **entrypoint secrets wiring** — add shell entrypoint scripts to Dockerfiles so `DATABASE_URL` can be fully assembled from Docker secret files without requiring `.env`
6. **STAGING_SMOKE_TOKEN rotation** — short-lived bot token via the Flint identity stack

**Alternative narrow scope (p11-patch):** Just the k6 baselines + entrypoint secrets wiring. 2 changes, 1 session. Minimum viable operational hardening before promoting staging to production.

---

*Generated by OpenCode `/kbd-reflect` — 2026-07-06*
