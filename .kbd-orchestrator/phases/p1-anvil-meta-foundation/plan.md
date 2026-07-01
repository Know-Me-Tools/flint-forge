# Phase Plan: p1-anvil-meta-foundation

**RFC:** RFC-FORGE-001 (§2-4), RFC-FORGE-META-001 (validated revision)
**Status:** plan_complete → ready for /kbd-execute
**Total Changes:** 11
**Estimated Batch Parallelism:** 4 batches

---

## Execution Order

### Batch 0 — Already Complete

| # | Change | Status |
|---|--------|--------|
| — | p1-c005-jwt-contract-pin | ✅ COMPLETE (docs/contracts/jwt-contract.md written) |

### Batch 1 — Parallel (no dependencies)

These four changes are fully independent. Execute all in parallel.

| # | Change | Description | Agent |
|---|--------|-------------|-------|
| 1 | p1-c001-flint-auth | Complete auth SQL helpers + tests | rust-reviewer |
| 2 | p1-c004-pg-cron | Add pg_cron to Dockerfile + config | devops-engineer |
| 3 | p1-c006-vault-kms | KMS docs + vault-init.sh + test | rust-reviewer |
| 4 | p1-c007-flint-meta-schema | New ext-flint-meta crate, all cache tables | rust-reviewer |

**Gate:** All four must pass `cargo pgrx test` (for pgrx changes) and `cargo check` (for workspace changes) before Batch 2.

**OQ-9 BLOCKER for c001:** Before executing p1-c002 (NOT c001), read `crates/ext-flint-hooks/Cargo.toml` to confirm pgrx version. p1-c001 targets pgrx 0.12/pg17 — no OQ dependency.

### Batch 2 — Sequential: meta build-up (depends on c007)

These three changes must execute in strict order — each builds on the previous.

| # | Change | Depends On | Description | Agent |
|---|--------|------------|-------------|-------|
| 5 | p1-c008-flint-meta-triggers | c007 | DDL event triggers + full_refresh() | rust-reviewer |
| 6 | p1-c009-flint-meta-functions | c007, c008 | tables(), columns(), relationships(), check_permission(), set_identity() | rust-reviewer |
| 7 | p1-c010-flint-meta-agui-descriptor | c007, c008, c009 | agui_descriptor() + openapi() JSONB functions | ai-engineer |

**Gate after c009:** `SELECT * FROM flint_meta.tables()` returns rows. `SELECT flint_meta.version()` returns ≥ 1.

**Gate after c010:** `SELECT jsonb_array_length(flint_meta.agui_descriptor()->'tools') > 0` — true with test tables present.

### Batch 3 — Parallel (depends on Batch 1 + Batch 2)

These run in parallel after all of Batch 1 and Batch 2 complete.

| # | Change | Depends On | Description | Agent |
|---|--------|------------|-------------|-------|
| 8 | p1-c002-flint-hooks-standard | c001 (auth), OQ-9 resolved | pg_net dispatch, HMAC signing | rust-reviewer |
| 9 | p1-c011-flint-meta-listener-test | c007, c008, c009 | sqlx PgListener phase gate test | tdd-guide |

**Gate for c002:** Resolve OQ-9 (read `crates/ext-flint-hooks/Cargo.toml`) immediately before execution.

### Batch 4 — Sequential (depends on Batch 3)

| # | Change | Depends On | Description | Agent |
|---|--------|------------|-------------|-------|
| 10 | p1-c003-flint-hooks-durable | c002 | BGW dispatcher, SKIP LOCKED, retry | rust-reviewer |

### Phase Gate (final)

**p1-c011 must pass before phase close:**
- `cargo test -p fdb-app --test meta_listener -- --nocapture` passes (or equivalent)
- Both tests pass: DDL notification within 5s, reconnect path confirmed
- `SELECT * FROM flint_meta.tables()` returns rows after CREATE TABLE

---

## Dependency Graph

```
p1-c005 ──────────────────────────────────────────── (already done)

p1-c001 ──────────────────────┐
p1-c004 ──────────────────────┤
p1-c006 ──────────────────────┤   (Batch 1: parallel)
p1-c007 ──────────────────────┤
                              │
              ┌───────────────┘
              │
         p1-c008 (needs c007)
              │
         p1-c009 (needs c007 + c008)
              │
         p1-c010 (needs c007 + c008 + c009)
              │
    ┌─────────┴──────────────────────┐
    │                                │
p1-c002 (needs c001 + OQ-9)    p1-c011 (needs c007+c008+c009)
    │
p1-c003 (needs c002)
```

---

## Open Questions (Deferred — resolve before indicated change)

| OQ | Blocker For | Action |
|----|-------------|--------|
| OQ-9 | p1-c002 (hooks standard) | Read `crates/ext-flint-hooks/Cargo.toml` to confirm pgrx version |
| OQ-10 | p1-c004 (pg_cron) | Read `images/postgres18/Dockerfile` for current pg_cron state before editing |
| OQ-3 | Phase 3 only | pg_graphql PG18 release — not blocking P1 |
| OQ-6 | Phase 7 only | FRF agentproto timeline — not blocking P1 |
| OQ-7 | Phase 7 only | ag-ui-client audit — not blocking P1 |
| OQ-8 | Phase 3 only | Keto sync via FRF Iggy — not blocking P1 |

---

## Security Constraints (apply to ALL changes)

These are non-negotiable and apply to every change in this phase:

1. **No `unwrap()`/`expect()` in library crates** — `thiserror` in libs, `anyhow` only in binary entry points (`fdb-gateway`, `fke-server`, `forge-cli`)
2. **Never log JWT payloads, claims, relation tuples, or tenant identifiers**
3. **`#[non_exhaustive]`** on all public enums
4. **Newtype IDs** as `#[repr(transparent)]` wrappers
5. **No file over 500 lines** — split into directory modules
6. **`clippy::pedantic` + `-D warnings`** — workspace `[lints]` applies
7. **`role` claim is NOT auto-injected** — every production route hook MUST explicitly add `"role": "authenticated"` or `"role": "service_role"` to `additional_claims` (see `docs/contracts/jwt-contract.md` §4)
8. **pgrx version split is intentional**: `ext-flint-auth` = pgrx 0.12/pg17; `ext-flint-vault` + `ext-flint-meta` = pgrx 0.18.1/pg18 — DO NOT unify

---

## Change Summary

| Change | Files Touched | Test Gate | Est. Complexity |
|--------|--------------|-----------|-----------------|
| p1-c001 | `sql/flint_auth.sql`, tests | `cargo pgrx test` pg17 | Low |
| p1-c002 | `sql/flint_hooks.sql`, pg_net dispatch | `cargo pgrx test` | Medium |
| p1-c003 | BGW Rust worker module | `cargo test`, BGW start | High |
| p1-c004 | `images/postgres18/Dockerfile` | Docker build | Low |
| p1-c005 | `docs/contracts/jwt-contract.md` | — (docs only) | ✅ Done |
| p1-c006 | `docs/vault-kms-guide.md`, `scripts/vault-init.sh` | doc review | Low |
| p1-c007 | New `crates/ext-flint-meta/` crate | `cargo pgrx test` pg18 | High |
| p1-c008 | `src/triggers.rs` in ext-flint-meta | `cargo pgrx test` pg18 | Medium |
| p1-c009 | `src/functions.rs` in ext-flint-meta | `cargo pgrx test` pg18 | Medium |
| p1-c010 | `src/descriptors.rs` in ext-flint-meta | `cargo pgrx test` pg18 | Medium |
| p1-c011 | `crates/fdb-app/tests/meta_listener.rs` | `cargo test` | Medium |

---

## Recommended Execution Notes

- **p1-c007 is the critical path** — the entire meta build-up (c008, c009, c010, c011) waits on it. Execute it as early as possible in Batch 1.
- **Use ext-flint-vault as the Cargo.toml template** for ext-flint-meta: pgrx = "=0.18.1", `crate-type = ["cdylib"]`, `default = ["pg18"]`, no `src/bin/pgrx_embed.rs`
- **p1-c003 (durable hooks BGW)** is the highest complexity change in this phase. Allocate extra review time.
- **p1-c011 is the phase gate** — do not close the phase until both PgListener tests pass.
- **OQ-10 must be resolved** before any Dockerfile edits in p1-c004 to avoid duplicating pg_cron if it's already present.

---

## Next Action

```
/kbd-execute p1-anvil-meta-foundation
```
