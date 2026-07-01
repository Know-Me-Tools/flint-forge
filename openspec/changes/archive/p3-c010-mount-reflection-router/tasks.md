# Tasks — p3-c010-mount-reflection-router

- [x] 1. Read `fdb-gateway/src/main.rs` and locate the `TODO(p2-c005)` site
- [x] 2. Read `fdb-reflection::router()` signature and confirm mount path
- [x] 3. Nest the reflection router under the Axum root router
- [x] 4. Add integration test `mounts_reflection_router` in `fdb-gateway/tests/`
- [x] 5. Run pre-flight: grep `flint_meta.sql` for `cedar_policies`; record finding
- [x] 6. `cargo check --workspace`
- [x] 7. `cargo clippy --workspace -- -D warnings`
- [x] 8. `cargo test -p fdb-gateway`
