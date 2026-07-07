# Plan — p14-v1.1.0

**Phase:** 14 — v1.1.0 Feature Cycle
**Authored:** 2026-07-07
**Change backend:** OpenSpec
**Changes:** 5 ordered
**Seeded from:** `assessment.md`

---

## Ordering Rationale

```
p14-c001-sqlx-prometheus (P0)
    └── changes Cargo.lock baseline; all other changes apply on top
            ↓
p14-c002-kiln-guest-sdk (P1)  ─┐ independent of each other;
p14-c003-a2ui-hot-reload (P1) ─┘ can run as parallel subagents
            ↓
p14-c005-kiln-metrics (P2)     small; same crate family as c001
p14-c004-jwt-rotation (P2)     shell script; fully independent
```

c001 must run first because the sqlx 0.9 upgrade changes the Cargo.lock for
all subsequent changes. c002 and c003 are fully independent (different crate
trees, no shared files) and can be dispatched as parallel subagents. c004 and
c005 are small and can be batched or parallelised.

---

## Change List

### 1. `p14-c001-sqlx-prometheus` — P0 — Execute first

**Scope:**
- Bump `sqlx = "0.9"` in workspace deps (6 crates affected)
- Verify pgvector `Encode`/`Type` unify across sqlx 0.9
- Run `cargo update generic-array` — confirm transitive conflict resolved
- Add pool metrics: `spawn_pool_metrics()` in `telemetry.rs`
- Add `metrics = "0.24"` to `fdb-gateway/Cargo.toml`

**Risk:** sqlx 0.8→0.9 API surface. Usage patterns (`PgPool::connect`,
`PgListener::connect_with`, `query_as`, `migrate!`) are stable across minor
versions. Main risk: `PgPoolOptions` API.

**Gate:** `cargo test --workspace`; `cargo audit`; `cargo update generic-array` succeeds.

---

### 2. `p14-c002-kiln-guest-sdk` — P1 — Parallel with c003

**Scope:**
- New `crates/flint-skill/` crate (~7 source files + tests + README)
- Typed wrappers: `Database`, `Llm`, `Kv`, `Identity`, `Secrets`
- `SkillError` with `thiserror`; JSON params via `serde_json::Value`
- No host-side changes — purely consumer-side

**Gate:** `cargo check -p flint-skill`; `cargo test -p flint-skill`; `cargo clippy -p flint-skill -- -D warnings`.

---

### 3. `p14-c003-a2ui-hot-reload` — P1 — Parallel with c002

**Scope:**
- Migration `0010_a2ui_change_notify.sql` — triggers on `flint_a2ui.components`
- `broadcast_all()` on `AgUiState` — iterates run channels
- Wire `subscribe_version()` → AG-UI `StateSnapshot` event
- `@flint/react` `useFlintRegistry()` auto-refresh on event

**Gate:** `cargo test --workspace`; `docker compose config --quiet`.

---

### 4. `p14-c005-kiln-metrics` — P2 — After c001

**Scope:**
- Add `axum-prometheus` + `metrics` to `fke-server`
- `/metrics` endpoint (same pattern as fdb-gateway)
- 3 Kiln-specific counters in `invoke_impl()`

**Gate:** `cargo test -p fke-server`; `/metrics` endpoint returns `kiln_invocations_total`.

---

### 5. `p14-c004-jwt-rotation-automation` — P2 — Independent

**Scope:**
- `scripts/rotate_staging_jwt.sh` — `openssl rand` + `gh secret set`
- `--dry-run` mode
- `scripts/README.md` + `docs/runbook.md` update

**Gate:** `bash -n` passes; `--dry-run` works.

---

## Build / Quality Gates

```bash
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo audit
bash scripts/check_api_versions.sh
```

---

## MVP Gate Checklist

- [ ] sqlx pool metrics emitted on `/metrics`
- [ ] `cargo update generic-array` succeeds without breaking pgvector
- [ ] `flint-skill` crate compiles
- [ ] A2UI hot-reload triggers re-compile + SSE notification
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean
