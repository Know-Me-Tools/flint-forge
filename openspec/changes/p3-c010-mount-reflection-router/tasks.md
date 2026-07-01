# Tasks — p3-c010-mount-reflection-router

- [ ] 1. Read `fdb-gateway/src/main.rs` and locate the `TODO(p2-c005)` site
- [ ] 2. Read `fdb-reflection::router()` signature and confirm mount path
- [ ] 3. Nest the reflection router under the Axum root router
- [ ] 4. Add integration test `mounts_reflection_router` in `fdb-gateway/tests/`
- [ ] 5. Run pre-flight: grep `flint_meta.sql` for `cedar_policies`; record finding
- [ ] 6. `cargo check --workspace`
- [ ] 7. `cargo clippy --workspace -- -D warnings`
- [ ] 8. `cargo test -p fdb-gateway`
