# Tasks — p3-c009-predicate-pushdown

## Change
Opt-in predicate pushdown for subscription WatchEntityType (P2 deferral)

## Status: PENDING (blocked on p3-c002; P2 priority — do not start before p3-c002 is production-verified)

## ⚠️ Pre-conditions before any work begins
- [ ] p3-c002 is complete and qa_passed
- [ ] Operator has acknowledged the data-leak risk in writing
- [ ] `docs/operations/predicate-pushdown.md` is drafted (T1 below)

---

## Task List

### T1 — Write risk documentation (prerequisite to all coding)
- [ ] Create `docs/operations/predicate-pushdown.md`:
  - What predicate pushdown is and why it is an optimization
  - **The risk**: FRF may deliver events that RLS would have filtered; the re-query
    remains active but a predicate mismatch creates a throughput vs. confidentiality tradeoff
  - When it is safe: homogeneous tenant subscriptions where the predicate matches
    the RLS policy exactly
  - When it is NOT safe: multi-tenant hot-swap, dynamic RLS policies, or any case
    where the application predicate may not match the DB RLS policy
  - How to enable: `FLINT_PREDICATE_PUSHDOWN=true`
  - How to verify it's working: log output at startup, `tracing` span attributes

### T2 — Add predicate field to SubscriptionSpec (fdb-ports)
- [ ] In `crates/fdb-ports/src/lib.rs`, in `SubscriptionSpec`:
  ```rust
  pub predicate: Option<String>,
  ```
- [ ] Update all `SubscriptionSpec` construction sites to set `predicate: None`
  (there should be 0–2 in the codebase at this point)

### T3 — Wire predicate in FabricChangeSource::watch() (fdb-realtime)
- [ ] In `crates/fdb-realtime/src/lib.rs`, in `FabricChangeSource::watch()`:
  - Read `FLINT_PREDICATE_PUSHDOWN` env var
  - If `true` AND `spec.predicate.is_some()`: include predicate in `WatchEntityTypeRequest`
  - If `false` OR `spec.predicate.is_none()`: omit predicate (FRF gets no filter)
  - Confirm FRF `WatchEntityTypeRequest` proto supports a `predicate` field — if not,
    log a one-time warning and skip (do not panic)

### T4 — Add startup warning in fdb-gateway
- [ ] In `crates/fdb-gateway/src/main.rs`, before `serve()`:
  ```rust
  if std::env::var("FLINT_PREDICATE_PUSHDOWN").as_deref() == Ok("true") {
      tracing::warn!("FLINT_PREDICATE_PUSHDOWN enabled — see docs/operations/predicate-pushdown.md");
  }
  ```

### T5 — Unit test: predicate not sent when flag off
- [ ] In `crates/fdb-realtime/src/lib.rs` `#[cfg(test)]`:
  - `test_predicate_not_sent_when_feature_flag_off`:
    - Ensure `FLINT_PREDICATE_PUSHDOWN` is NOT set in test env
    - Build a `SubscriptionSpec` with `predicate: Some("tenant_id = 'abc'".to_string())`
    - Call `FabricChangeSource::watch()` (mocked FRF client)
    - Assert the captured `WatchEntityTypeRequest` has `predicate: None` (predicate suppressed)
  - `test_predicate_sent_when_feature_flag_on`:
    - Set `FLINT_PREDICATE_PUSHDOWN=true` in test env
    - Same setup
    - Assert `WatchEntityTypeRequest.predicate == Some("tenant_id = 'abc'")`

### T6 — Verify per-event RLS re-query still runs
- [ ] Run the existing test from p3-c002 T6 (`test_watch_drops_rls_filtered_events`)
  with `FLINT_PREDICATE_PUSHDOWN=true` set — assert it still passes (re-query
  still drops events not returned by RLS)
- [ ] This test proves predicate pushdown does not bypass the security re-query

### T7 — Compile and lint gate
- [ ] `cargo check --workspace` — GREEN
- [ ] `cargo clippy --workspace -- -D warnings` — GREEN
- [ ] `cargo test --workspace` — all tests pass
- [ ] Mark `p3-c009` as `qa_passed` in `progress.json`
