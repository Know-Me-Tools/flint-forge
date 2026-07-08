# Plan — p15-v1.0-production-readiness

**Phase:** 15 — v1.0 Production Readiness Gap Closure
**Authored:** 2026-07-07
**Change backend:** OpenSpec
**Changes:** 5 ordered
**Seeded from:** Assessment directive + user decisions

---

## User Decisions Applied

1. **pgrx target:** pgrx `0.18.1` + Postgres `pg18`; bump Rust toolchain to
   `>= 1.96` for extension builds.
2. **Staging host:** Local Colima Docker machine for k6 baselines.
3. **CLI packaging:** BOTH static binary and container image; runtime mode
   selected by command / env.
4. **Production packaging:** Include a minimal Helm chart in v1.0 (Docker
   Compose remains the dev/staging path).

---

## Ordering Rationale

```
p15-c001-anvil-extension-stabilization (P0) ─┐ independent; both block boot
p15-c002-migration-integrity (P0)            ─┘
            ↓
p15-c003-operator-cli (P1)                    depends on c001/c002 being green
            ↓
p15-c004-e2e-performance-validation (P1)      exercises CLI + migrations + extensions
            ↓
p15-c005-docs-production-artifacts (P2)       final polish + Helm chart
```

- **c001 and c002 are P0 and can run in parallel.** They are the only true
  blockers for a v1.0 boot sequence.
- **c003** needs the migration and extension paths to be stable so the CLI can
  delegate to them confidently.
- **c004** validates the integrated system (CLI + migrations + extensions +
  runtime) and produces the performance contract.
- **c005** is final packaging and documentation; it must come last so it can
  accurately describe the shipped state.

---

## Change List

### 1. `p15-c001-anvil-extension-stabilization` — P0

**Scope:**
- Bump workspace/toolchain Rust requirement to `>= 1.96` for pgrx 0.18.1.
- Upgrade `ext-flint-auth` and `ext-flint-hooks` from pgrx 0.12 to 0.18.1.
- Remove workspace inheritance from all five pgrx crates (or create a pgrx
  workspace overlay) so they build standalone with `cargo pgrx`.
- Fix `DatumWithOid` usage in `ext-flint-meta/src/functions.rs` (replace with
  pgrx 0.18.1 equivalent).
- Align all extension `Cargo.toml` files to use the same pgrx/pg18 defaults.
- Resolve macOS linker configuration for local dev; validate Linux build in CI.
- Add a pgrx CI job that runs `cargo pgrx test` in a Linux container with
  Postgres 18.

**Risk:** pgrx 0.18.1 API surface may require more changes than `DatumWithOid`
once compilation begins. The upgrade from 0.12 to 0.18.1 for auth/hooks is a
major-version-equivalent jump.

**Gate:** `cargo pgrx test` passes for all five extensions in CI.

---

### 2. `p15-c002-migration-integrity` — P0

**Scope:**
- Renumber colliding migrations:
  - `0005_cedar_policies.sql` and `0005_flint_a2ui_hybrid_search.sql`
  - `0006_change_notify.sql` and `0006_flint_a2ui_application_model.sql`
- Produce a strict `0001..0010` sequence (or higher as needed).
- Add CI step that starts a Postgres 18 container and runs
  `sqlx migrate run` / `sqlx migrate info` against an empty database.
- Add integration test that exercises `fdb-gateway` startup migration path
  (`crates/fdb-gateway/src/main.rs:95-98`).
- Update runbook with migration renumbering guidance for operators upgrading
  from earlier snapshots.

**Risk:** Renumbering can break existing developer databases. The CI step and
runbook note mitigate this; production environments should be rebuilt for v1.0.

**Gate:** `sqlx migrate run` succeeds against an empty Postgres 18 database in
CI.

---

### 3. `p15-c003-operator-cli` — P1

**Scope:**
- Replace `forge-cli` stub with a real CLI using `clap`:
  - `forge version`
  - `forge fn register <path>` — register a WASM component via Kiln admin API
  - `forge hook add <table> <url>` — add a webhook dispatch rule via Quarry API
  - `forge migrate` — apply SQL migrations (delegate to `sqlx-cli` or psql)
  - `forge token mint` — mint a smoke-test JWT (port logic from
    `scripts/mint_smoke_token.sh`)
- Add `--container` / `FORGE_CONTAINER=1` mode that runs the same command
  inside the `flint-forge-cli` container image.
- Add Dockerfile for `forge-cli` container image.
- Add unit tests for argument parsing and command delegation.
- Wire CLI build and tests into CI.

**Risk:** Scope creep into a full control-plane CLI. Keep v1.0 surface minimal:
  the five subcommands above only.

**Gate:** `forge --help` lists working subcommands; `cargo test -p forge-cli`
passes; container image builds in CI.

---

### 4. `p15-c004-e2e-performance-validation` — P1

**Scope:**
- Add CI job that starts Postgres 18 + extensions + `fdb-gateway` + `fke-server`
  in a Colima/Docker Compose local stack.
- Run `DATABASE_URL`-gated integration tests against the real database.
- Gate tests that need external services (FRF, Keto) behind feature flags or
  `#[ignore]` with clear labels.
- Implement `sqlx_pool_connections_open` metric:
  - Option A: custom pool listener on deadpool-postgres
  - Option B: bump to sqlx 0.9 if pgrx/toolchain work unblocks it
- Run k6 baselines against the Colima stack using `perf/k6/regression.js`.
- Populate `docs/performance.md` with measured P50/P95/P99 for key endpoints.

**Risk:** k6 results on Colima/macOS may not match Linux production numbers.
Document them as "local baseline" and leave cloud baselines as a follow-up.

**Gate:** CI integration tests pass against Postgres; k6 regression thresholds
committed; `/metrics` exposes `sqlx_pool_connections_open`.

---

### 5. `p15-c005-docs-production-artifacts` — P2

**Scope:**
- Update `README.md`:
  - Change status from "scaffold" to "v1.0-ready" or equivalent.
  - Remove references to `todo!()` stubs.
  - Update active-phase pointer from p3/p5 to current state.
- Refresh `docs/security-audit.md`:
  - Correct the `cargo audit` CI claim.
  - Add current extension/CLI threat model notes.
- Refresh `docs/ROADMAP.md`:
  - Mark completed items (e.g., `flint-skill`, JWT rotation, Kiln metrics).
  - Move deferred items (sqlx 0.9, multi-tenant isolation) to a future cycle.
- Add minimal Helm chart under `deploy/helm/flint-forge/`:
  - `Deployment` for `fdb-gateway` and `fke-server`
  - `Service`, `ConfigMap`, `Secret`
  - `StatefulSet` or dependency note for Postgres + extensions
  - `helm lint` passes
- Fix hard-coded Kiln cache miss:
  - Expose `is_loaded()` on the component registry/store adapter.
  - Update `crates/fke-server/src/main.rs:205` to use it.
  - Add test for cache hit path.

**Risk:** Helm chart scope can expand. Keep it minimal — no operator controller,
no complex ingress templating.

**Gate:** README/ROADMAP/security-audit are internally consistent; `helm lint`
passes; Kiln cache hit path is exercised in tests.

---

## Build / Quality Gates

```bash
# Non-pgrx workspace
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace

# pgrx extensions (CI container)
cargo pgrx test -p ext-flint-auth
cargo pgrx test -p ext-flint-hooks
cargo pgrx test -p ext-flint-llm
cargo pgrx test -p ext-flint-meta
cargo pgrx test -p ext-flint-vault

# Migrations
docker compose -f docker-compose.yml up -d postgres
sqlx migrate run

# CLI container
docker build -t flint-forge-cli -f crates/forge-cli/Dockerfile .

# Helm
cd deploy/helm/flint-forge && helm lint .

# k6 baselines (Colima)
docker compose -f docker-compose.staging.yml up -d
k6 run perf/k6/regression.js
```

---

## MVP Gate Checklist

- [ ] All five Anvil pgrx extensions compile and pass `cargo pgrx test` in CI.
- [ ] `sqlx migrate run` succeeds against an empty Postgres 18 database.
- [ ] `forge-cli` implements and tests all documented subcommands.
- [ ] CI runs integration tests against a real Postgres instance.
- [ ] k6 performance baselines are committed with real thresholds.
- [ ] README, security-audit, and ROADMAP docs are internally consistent.
- [ ] Helm chart is present and lints cleanly.
- [ ] `cargo test --workspace` and `cargo clippy --workspace -- -D warnings` remain green.

---

## 3-Wait Budget

This phase will wait for tests at most 3 times:

1. After c001 + c002 land — run `cargo pgrx test` + `sqlx migrate run` integration.
2. After c003 + c004 land — run full local stack + integration tests + k6 baseline.
3. Final green run after c005 — full workspace check + Helm lint + docs review.

Record wait-count in `.kbd-orchestrator/phases/p15-v1.0-production-readiness/progress.json`.

---

## Decision Required

Approve this plan as the v1.0 readiness phase. If scope must be reduced, the
minimum shippable subset is **p15-c001 + p15-c002**; everything else can be
flagged as post-v1.0 follow-up.

---

*Generated by OpenCode `/kbd-plan` — 2026-07-07*
