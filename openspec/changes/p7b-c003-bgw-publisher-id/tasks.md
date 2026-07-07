# p7b-c003 Tasks — BGW Publisher Identity

## Tasks

- [ ] Add `forge-identity = { path = "../forge-identity" }` to `fke-server/Cargo.toml`
- [ ] Add `fn publisher_rls(manifest: &FunctionManifest) -> RlsContext` to `kiln_bgw.rs`
- [ ] In `invoke_function()`: declare `let publisher = publisher_rls(&manifest);`
- [ ] Pass `Some(&publisher)` as `caller` to both `runtime.handle()` calls in `invoke_function()`
- [ ] Remove the `// BGW = system caller; Cedar gate is skipped` comment (or update it)
- [ ] Unit test: `publisher_rls()` sets `keto_subject = publisher_did` and `role = "kiln_publisher"`
- [ ] Unit test: `publisher_rls()` sets `raw_bearer = ""`
- [ ] `cargo clippy -p fke-server -- -D warnings` clean
- [ ] `cargo test -p fke-server` passes
