# p6b-c003 — WIT Contract Freeze + wasi:http Dispatch

**Phase:** 6b — Kiln Hardening
**Priority:** P0
**Depends on:** p6b-c001 (Cedar gate), p6-c001 (fke-runtime)

## What this change delivers

Freezes the `flint:host@0.1.0` WIT interface, wires `wasmtime-wasi-http` into
`fke-runtime`, and replaces the stub response in `EdgeRuntime::handle()` with a
real `wasi:http/incoming-handler.handle()` dispatch.

## Design

### `crates/fke-domain/wit/flint-host.wit`

```wit
package flint:host@0.1.0;

world kiln-edge {
  import wasi:http/incoming-handler@0.2.0;
  import wasi:http/types@0.2.0;
  import wasi:io/streams@0.2.0;
  export wasi:http/incoming-handler@0.2.0;
}
```

### `crates/fke-runtime/Cargo.toml` additions

```toml
wasmtime-wasi-http = { workspace = true }

[build-dependencies]
wit-bindgen = "0.44"
```

### `crates/fke-runtime/build.rs`

```rust
fn main() {
    println!("cargo:rerun-if-changed=../fke-domain/wit/");
}
```

### Updated `EdgeRuntime::handle()`

Replace:
```rust
let _ = (instance, request);
Ok(KilnResponse { status: 200, body: b"ok".to_vec() })
```

With the real `wasi:http/incoming-handler` typed call via the generated
`InstancePre<KilnHostState>` + `WasiHttpView` implementation.

## Constraint note

`wasmtime-wasi-http = "26"` must be added to `[workspace.dependencies]` in
`Cargo.toml`. The `KilnHostState` must also implement `WasiHttpView`.

## Gate

`examples/hello-component` must build and run with the updated `fke-runtime`
after this change lands.
