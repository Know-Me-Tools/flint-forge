# p6b-c003 Tasks — WIT Contract + wasi:http Dispatch

## Tasks

- [ ] Create `crates/fke-domain/wit/flint-host.wit` — `package flint:host@0.1.0`, world `kiln-edge` with `import wasi:http/incoming-handler@0.2.0` and matching exports
- [ ] Add `wasmtime-wasi-http = "26"` to `[workspace.dependencies]` in `Cargo.toml`
- [ ] Add `wasmtime-wasi-http = { workspace = true }` to `fke-runtime/Cargo.toml`
- [ ] Add `[build-dependencies] wit-bindgen = "0.44"` to `fke-runtime/Cargo.toml`
- [ ] Create `crates/fke-runtime/build.rs` — emit `rerun-if-changed` for `../fke-domain/wit/`
- [ ] Implement `WasiHttpCtx` and `WasiHttpView` for `KilnHostState` in `fke-runtime/src/lib.rs`
- [ ] Add `wasmtime_wasi_http::add_only_http_to_linker_async(&mut linker)` in `build_linker()`
- [ ] Replace the stub response in `EdgeRuntime::handle()` with real `wasi:http/incoming-handler` typed dispatch via the generated interface
- [ ] Add `KilnRequest → wasmtime_wasi_http::IncomingRequest` conversion helper
- [ ] Verify `examples/hello-component` still compiles and its test passes
- [ ] Gate test: load `examples/hello-component` WASM artifact and call `handle()` — verify HTTP 200 response
- [ ] `cargo clippy -p fke-runtime -p fke-domain -- -D warnings` clean
