# Execution Contract: p1-anvil-meta-foundation

**Backend:** `openspec` (Claude Code as execution engine)  
**Dispatch contract version:** 1  
**Stage gate:** PASSED (plan handoff exists)  
**Written:** 2026-06-30  
**Changes:** 11 total, 1 already done (p1-c005)

---

## Open Questions Resolved at Execute-Time

### OQ-9 — RESOLVED
`crates/ext-flint-hooks/Cargo.toml`: `pgrx = "0.12"`, features `pg16` + `pg17`.  
**Consequence:** p1-c002 (hooks standard) targets pgrx 0.12/pg17. DO NOT migrate to 0.18.1.

### OQ-10 — RESOLVED
`images/postgres18/Dockerfile`: no pg_cron present. `shared_preload_libraries=pg_net` only.  
**Consequence:** p1-c004 must:
1. Add a pg_cron build stage (or apt install where available for pg18)
2. Append `pg_cron` to `shared_preload_libraries` in the CMD line: `pg_net,pg_cron`
3. Add pg_cron schema registration in `init/01-extensions.sql`

---

## Dispatch Order

### DONE — Batch 0

| Change | Status | Artifact |
|--------|--------|----------|
| p1-c005 — jwt-contract-pin | ✅ done | `docs/contracts/jwt-contract.md` |

### READY — Batch 1 (all parallel)

Execute all four simultaneously. No inter-dependencies.

| Priority | Change | openspec path | Implements |
|----------|--------|---------------|------------|
| **CRITICAL PATH** | p1-c007-flint-meta-schema | `openspec/changes/p1-c007-flint-meta-schema/` | New ext-flint-meta crate + all cache tables |
| High | p1-c001-flint-auth | `openspec/changes/p1-c001-flint-auth/` | auth SQL tests + role-fallback + auth.tenant_id() + schema lockdown |
| High | p1-c004-pg-cron | `openspec/changes/p1-c004-pg-cron/` | pg_cron Dockerfile stage + preload + cron registration |
| Medium | p1-c006-vault-kms | `openspec/changes/p1-c006-vault-kms/` | Azure KMS guide + vault-init.sh + additional test |

**Gate:** All four: `cargo pgrx test` passes (pgrx crates), `cargo check --workspace` passes.

### BLOCKED on Batch 1 — Batch 2 (sequential: must execute in order)

| Order | Change | Waits for | Implements |
|-------|--------|-----------|------------|
| 1st | p1-c008-flint-meta-triggers | p1-c007 | DDL event triggers + full_refresh() |
| 2nd | p1-c009-flint-meta-functions | p1-c007 + p1-c008 | Reflection query functions + check_permission() + set_identity() |
| 3rd | p1-c010-flint-meta-agui-descriptor | p1-c007 + p1-c008 + p1-c009 | agui_descriptor() + openapi() JSONB |

**Intermediate gate after c009:** `SELECT * FROM flint_meta.tables()` returns rows. `SELECT flint_meta.version()` ≥ 1.

### BLOCKED on Batch 1+2 — Batch 3 (parallel)

| Change | Waits for | Implements |
|--------|-----------|------------|
| p1-c002-flint-hooks-standard | p1-c001 + OQ-9 resolved | pg_net dispatch + HMAC-SHA256 signing |
| p1-c011-flint-meta-listener-test | p1-c007 + p1-c008 + p1-c009 | sqlx PgListener phase gate tests |

### BLOCKED on Batch 3 — Batch 4 (sequential)

| Change | Waits for | Implements |
|--------|-----------|------------|
| p1-c003-flint-hooks-durable | p1-c002 | BGW dispatcher + SKIP LOCKED retry |

---

## Phase Gate (Required Before Close)

`p1-c011` PgListener tests:
```
cargo test -p fdb-app --test meta_listener -- --nocapture
```
Both must pass:
1. `meta_listener_receives_notify_on_create_table` — NOTIFY received within 5s
2. `meta_listener_reconnect_forces_recompile` — reconnect + re-LISTEN confirmed

---

## Security Constraints (all changes, non-negotiable)

1. No `unwrap()`/`expect()` in library crates
2. No JWT payload logging anywhere
3. `role` claim MUST be explicit in `additional_claims` — never auto-injected
4. pgrx 0.12/pg17 for `ext-flint-hooks` (hooks) — DO NOT migrate
5. pgrx 0.18.1/pg18 for `ext-flint-meta` and `ext-flint-vault` — DO NOT unify
6. `ext-flint-vault` `Cargo.toml` is the template for `ext-flint-meta`
7. No file > 500 lines — split into directory modules
8. `clippy::pedantic` + `-D warnings` on all workspace crates

---

## Artifact-Refiner QA Gate

Apply after each change with ≥3 files modified:

- Read constraints from `.kbd-orchestrator/constraints.md`
- Validate all produced artifacts
- Write `.refiner/artifacts/<change-id>/refinement_log.md`
- ALL PASS → mark `done` in progress.json, archive via OpenSpec
- ANY FAIL → mark `blocked`, iterate

**Skip QA for:** p1-c005 (already done), p1-c006 (docs only, <3 files)

---

## First Dispatch Command

Execute Batch 1 starting with the critical path:

```
Starting change 2 of 11: p1-c007-flint-meta-schema
```

Tasks per `openspec/changes/p1-c007-flint-meta-schema/tasks.md`
