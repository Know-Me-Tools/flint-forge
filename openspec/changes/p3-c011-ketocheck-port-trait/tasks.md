# Tasks — p3-c011-ketocheck-port-trait

- [ ] 1. Add `KetoCheck` trait to `fdb-ports/src/` (new file `keto.rs`)
- [ ] 2. Export from `fdb-ports/src/lib.rs`
- [ ] 3. Implement `KetoCacheAdapter` in `fdb-gateway::keto_sync`
- [ ] 4. Inject `Arc<dyn KetoCheck>` into `Quarry` mutation use-cases (composition in gateway)
- [ ] 5. Wire `check()` call into mutation path; return typed 403 on false
- [ ] 6. Add unit test mock `MockKetoCheck` in `fdb-app/tests/` or `fdb-app` test module
- [ ] 7. Audit tracing spans — no subject/relation logged
- [ ] 8. `cargo check --workspace` + clippy + `cargo test -p fdb-ports -p fdb-app`
