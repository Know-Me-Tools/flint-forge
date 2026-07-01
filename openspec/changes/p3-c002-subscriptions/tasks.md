# Tasks — p3-c002-subscriptions

## Change
FabricChangeSource: tonic + Keto gate + per-event RLS re-query

## Status: PENDING (blocked on p3-c004 + OQ-FRF-1 resolution)

---

## Task List

### T0 — Resolve OQ-FRF-1: WatchEntityType proto (prerequisite)
- [ ] Check `/Users/gqadonis/Projects/prometheus/flint-realtime-fabric/` for `.proto` files
- [ ] Locate `WatchEntityType` RPC definition
- [ ] Determine: generated Rust client available, or must run `tonic-build` from `.proto`?
- [ ] Document resolution in `current-waypoint.json` OQ list (remove OQ-FRF-1 once resolved)

### T1 — Add tonic to workspace dependencies
- [ ] In root `Cargo.toml` `[workspace.dependencies]`, add:
  ```toml
  tonic = "0.12"
  prost = "0.13"
  ```
- [ ] In `crates/fdb-realtime/Cargo.toml`, add:
  ```toml
  tonic = { workspace = true }
  fdb-ports = { path = "../fdb-ports" }
  deadpool-postgres = { workspace = true }   # for db re-query pool
  reqwest = { workspace = true }             # for Keto HTTP check
  tokio-stream = "0.1"                       # for wrapping gRPC stream as BoxStream
  ```

### T2 — Wire FRF proto client
- [ ] Copy or reference the generated tonic client for `WatchEntityType` from `flint-realtime-fabric`
  - If generated: copy to `crates/fdb-realtime/src/proto/` and add `tonic::include_proto!` or direct mod
  - If source .proto: add `build.rs` with `tonic_build::compile_protos("../../flint-realtime-fabric/proto/watch.proto")`
- [ ] Verify `WatchEntityTypeRequest` and `EntityChange` types are accessible

### T3 — Implement FabricChangeSource struct fields
- [ ] In `crates/fdb-realtime/src/lib.rs`, replace struct comment placeholder with real fields:
  ```rust
  pub struct FabricChangeSource {
      channel: tonic::transport::Channel,
      keto_base_url: String,
      http: reqwest::Client,
      db: Arc<dyn fdb_ports::DatabaseBackend>,
  }
  ```
- [ ] Add a `FabricChangeSource::new(fabric_url: &str, keto_url: &str, db: Arc<dyn DatabaseBackend>) -> Self` constructor

### T4 — Implement keto_check helper
- [ ] Write `async fn keto_check(subject: &str, verb: &str, object: &str, base_url: &str, client: &reqwest::Client) -> Result<(), StreamError>`
- [ ] Call Keto `/relation-tuples/check` endpoint (Ory Keto v2 API)
- [ ] Map: 200 → Ok(()); non-200 → Err(StreamError::Denied); network error → Err(StreamError::Unavailable)
- [ ] `#[instrument(skip(subject, client), fields(object, verb), err)]` — do NOT log subject (PII)

### T5 — Implement FabricChangeSource::watch() body
- [ ] Replace `todo!("fabric WatchEntityType + Keto gate + per-event RLS re-query")` with:
  1. Call `keto_check(who.keto_subject.as_str(), "view", &spec.entity_type, ...)` — return `Err` if denied
  2. Connect tonic stream: `WatchEntityTypeClient::new(channel).watch_entity_type(request).await`
  3. Map gRPC stream via `tokio_stream::StreamExt::map`:
     - For each `EntityChange`: call `db.acquire(who)`, run re-query, yield `Ok(ChangeEvent)` or skip on empty
     - On error: yield `Err(StreamError::Unavailable)`
  4. Return the mapped stream as `BoxStream<'static, Result<ChangeEvent, StreamError>>`
- [ ] The re-query table name MUST be built from `spec.entity_type` validated against a whitelist (not free input)

### T6 — Unit test: per-event RLS re-query drops filtered events
- [ ] In `crates/fdb-realtime/src/lib.rs` `#[cfg(test)]`:
  - Mock `DatabaseBackend` to return `None` (RLS-filtered empty result) for re-query
  - Assert that no `ChangeEvent` is yielded when re-query returns empty
  - (Use `mockall` or a manual mock struct implementing `DatabaseBackend`)
- [ ] Run `cargo test -p fdb-realtime` — pass

### T7 — Compile and lint gate
- [ ] `cargo check --workspace` — GREEN
- [ ] `cargo clippy --workspace -- -D warnings` — GREEN
- [ ] `cargo test --workspace` — all tests pass
- [ ] Mark `p3-c002` as `qa_passed` in `progress.json`
