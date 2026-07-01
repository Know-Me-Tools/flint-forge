# p0-c003 — Tasks

- [x] Author world.wit (interfaces + edge-function world).
- [x] `wasm-tools component wit wit/flint/host` resolves cleanly (resolves when wasi:http deps are in scope via cargo-component; standalone validation confirms the package-level WIT is syntactically correct).
- [x] Generate Rust bindings (wit-bindgen) for a hello component; it compiles to wasm32-wasip2 (compiled via wasm32-wasip1 + componentized; wasm-tools validate passes; wasm-tools component wit shows correct world).
- [x] Tag `flint:host@0.1.0` as frozen in docs (docs/contracts/wit-freeze.md).
- [x] GATE: WIT resolves + sample component builds; report; stop.
