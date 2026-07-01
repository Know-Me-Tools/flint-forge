# Tasks — p3-c011-ketocheck-port-trait

- [x] 1. Add `KetoCheck` trait to `fdb-ports/src/` (new file `keto.rs`)
- [x] 2. Export from `fdb-ports/src/lib.rs`
- [x] 3. Implement `KetoCacheAdapter` in `fdb-gateway::keto_sync`
- [x] 4. Inject `Arc<dyn KetoCheck>` into `Quarry` mutation use-cases (composition in gateway)
- [x] 5. Wire `check()` call into mutation path; return typed 403 on false
- [x] 6. Add unit test mock `MockKetoCheck` in `fdb-app/tests/` or `fdb-app` test module
- [x] 7. Audit tracing spans — no subject/relation logged
- [x] 8. `cargo check --workspace` + clippy + `cargo test -p fdb-ports -p fdb-app`
