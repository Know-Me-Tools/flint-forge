# p0-c003 — Freeze flint:host WIT contract

## Why
The edge-function capability surface must be frozen before SDKs, bindings, or the Cedar-gated
linker are built against it. Changing it later breaks every published component.

## What
- `wit/flint/host/world.wit`: interfaces `db`, `llm`, `kv`, `identity`, `secrets`;
  world `edge-function` exporting `wasi:http/incoming-handler`, importing `wasi:http/outgoing-handler`.
- Pin WASI to 0.2 now (note 0.3 `wasi:http/service` migration in spec §8).

## Contract
The world resolves with `wasm-tools`/`wit-bindgen`; a trivial Rust component targeting it
compiles. Version is `flint:host@0.1.0` and is treated as frozen.
