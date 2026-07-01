# Tasks — p3-c008-extended-guc-propagation

## Change
Extend SET LOCAL block + RlsContext fields + close Phase 2 instrument security debt

## Status: PENDING

---

## Task List

### T1 — Fix Phase 2 security debt: instrument skip
- [ ] In `crates/forge-identity/src/lib.rs` line 61, add `#[instrument(skip(bearer), err)]` to `verify_and_build()`
- [ ] Verify `tracing` is already imported (it is — check `forge-identity/Cargo.toml`)
- [ ] Run `cargo check -p forge-identity` to confirm

### T2 — Extend `RlsContext` struct
- [ ] In `crates/forge-identity/src/lib.rs`, add `keto_subject: String` and `vault_key_id: Option<String>` to `RlsContext`
- [ ] Update all `RlsContext { ... }` struct literals in `verify_and_build()` to populate new fields:
  - `keto_subject = claims.sub.clone()`
  - `vault_key_id = claims.extra.get("vault_key_id").and_then(|v| v.as_str()).map(String::from)`

### T3 — Unit tests for new RlsContext fields
- [ ] In `crates/forge-identity/src/lib.rs` `#[cfg(test)]` module:
  - [ ] `test_rls_context_keto_subject_from_claims` — construct RlsContext manually; assert `keto_subject == sub`
  - [ ] `test_rls_context_vault_key_id_from_extra_claim` — construct RlsContext with vault_key_id; assert Some("...")
  - [ ] `test_rls_context_vault_key_id_absent` — construct without vault_key_id claim; assert None
- [ ] Run `cargo test -p forge-identity` — all pass

### T4 — Add 3 extended SET LOCAL statements to PgBackend::acquire()
- [ ] In `crates/fdb-postgres/src/lib.rs`, after the existing 3 SET LOCAL calls, add:
  - `SET LOCAL "app.jwt_claims" = $1` (same value as `request.jwt.claims`)
  - `SET LOCAL "app.keto_subject" = $1` (from `rls.keto_subject`)
  - `SET LOCAL "app.vault_key_id" = $1` (from `rls.vault_key_id.as_deref().unwrap_or("")`)
- [ ] All 3 new statements use `PgError::SetLocal(...)` error mapping consistent with existing 3
- [ ] Verify all 6 SET LOCAL statements remain inside the same `BEGIN` transaction (no COMMIT between them)

### T5 — Compile and lint gate
- [ ] `cargo check --workspace` — GREEN
- [ ] `cargo clippy --workspace -- -D warnings` — GREEN
- [ ] `cargo test --workspace` — all 9 existing tests still pass
- [ ] Mark `p3-c008` as `qa_passed` in `progress.json`
