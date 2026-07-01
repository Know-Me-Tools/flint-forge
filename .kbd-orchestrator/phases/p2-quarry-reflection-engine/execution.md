# Execution — p2-quarry-reflection-engine

**Date:** 2026-06-30  
**Backend:** `openspec` — all 7 changes tracked in `openspec/changes/p2-c*/`  
**Dispatch:** `/kbd-apply p2-quarry-reflection-engine`  
**QA Gate:** artifact-refiner per-change (≥3 files modified — applies to all 7 changes)

---

## Selected Backend: OpenSpec

OpenSpec is available at `openspec/` (confirmed). All change proposals and
task files exist at `openspec/changes/p2-c*/proposal.md` + `tasks.md`.
Traceability and change archiving will use OpenSpec conventions.

---

## Dispatch Contract

### Phase Gate

**PASS criteria (all must be true):**
1. RLS-correct REST CRUD under a real flint-gate JWT (HTTP 200, correct rows returned)
2. ArcSwap hot-swap completes within 5s of DDL change notification
3. Zero dropped requests (no HTTP 500) during schema reload under 100 concurrent clients
4. `cargo check --workspace` passes (no warnings)
5. `cargo clippy --workspace -- -D warnings` passes
6. `test_vault_dek_not_in_compiled_state` gate test passes (security invariant)

### Change Execution Order

```
Phase 1 of 4: MVP P0 scaffold
  Change 1 of 7: p2-c003-flint-reflection-crate  (NEW crate — serial first)

Phase 2 of 4: MVP P0 auth + pool  
  Change 2 of 7: p2-c001-fdb-auth        ─┐ (parallel)
  Change 3 of 7: p2-c002-fdb-postgres     ─┘

Phase 3 of 4: MVP P0 compiler + hot-reload
  Change 4 of 7: p2-c004-rest-compiler
  Change 5 of 7: p2-c005-arcswap-hot-reload

Phase 4 of 4: P1 post-MVP
  Change 6 of 7: p2-c007-openapi-compiler
  Change 7 of 7: p2-c006-pgvector-rpc
```

### Per-Change QA Gate

After each change is marked DONE in `progress.json`:
1. Run `/refine-validate <change-id>` against `.kbd-orchestrator/constraints.md`
2. ALL PASS → mark `status: "qa_passed"` in `progress.json`, proceed to next change
3. ANY FAIL → mark `status: "blocked"`, run `/refine-code <change-id>`, recheck

Skip QA only if the change modifies fewer than 3 files or is docs-only.
All 7 Phase 2 changes modify ≥3 files — QA applies to all.

---

## Workspace Mutations (ordered)

These workspace-level changes must land before the crates that consume them:

| When | Change | `Cargo.toml` mutation |
|---|---|---|
| With p2-c003 | Register `fdb-reflection` | Add `"crates/fdb-reflection"` to `[workspace] members` |
| With p2-c003 | Add `sqlx` | `sqlx = { version = "0.8", features = ["postgres", "runtime-tokio", "uuid", "json"] }` |
| With p2-c001 | Add `jsonwebtoken` | `jsonwebtoken = "9"` |
| With p2-c001 | Add `reqwest` | `reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }` |
| With p2-c002 | Add `deadpool-postgres` | `deadpool-postgres = "0.14"` |
| With p2-c002 | Add `tokio-postgres` | `tokio-postgres = "0.7"` |
| With p2-c006 | Add `pgvector` | `pgvector = { version = "0.4", features = ["sqlx"] }` |

---

## Security Gates (enforced per-change, not just at phase end)

| Gate | Checked by |
|---|---|
| No JWT values in `tracing` spans | `constraints.md` BLOCK rule; unit test: tracing subscriber capture |
| `EncryptedDek` ciphertext only | `test_encrypted_dek_contains_no_plaintext` in p2-c003 |
| `SET LOCAL` inside `Transaction` | Integration test: GUC not visible on fresh acquire |
| Column names validated vs `DatabaseModel` | `test_rest_unknown_column_returns_400` in p2-c004 |
| `role` absent → `"anon"` | `test_missing_role_coerces_to_anon` in p2-c001 |
| `fdb-reflection` ≠ `fdb-gateway` | `cargo tree -p fdb-reflection` check in p2-c003 T11 |

---

## Progress Tracking

Live status in `.kbd-orchestrator/phases/p2-quarry-reflection-engine/progress.json`.

Update per change:
- `status: "in_progress"` when starting
- `status: "done"` when tasks complete
- `status: "qa_passed"` after artifact-refiner pass
- `status: "blocked"` on QA failure

---

## Artifacts to Produce

| Change | Key Artifacts |
|---|---|
| p2-c003 | `crates/fdb-reflection/` (entire crate), updated root `Cargo.toml` |
| p2-c001 | `crates/forge-identity/src/jwks.rs`, `error.rs`, updated `lib.rs` |
| p2-c002 | `crates/fdb-postgres/src/conn.rs`, `error.rs`, updated `lib.rs` |
| p2-c004 | `crates/fdb-reflection/src/compilers/rest.rs`, `handlers.rs` |
| p2-c005 | `crates/fdb-reflection/src/state_manager.rs`, updated `fdb-gateway/src/main.rs` |
| p2-c007 | `crates/fdb-reflection/src/compilers/openapi.rs`, `GET /openapi.json` route |
| p2-c006 | Updated `compilers/handlers.rs` with vector binding, `pgvector` dep |
