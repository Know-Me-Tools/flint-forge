# p14-c002 Tasks — Kiln Guest Rust SDK

## Tasks

- [x] Create `crates/flint-skill/Cargo.toml` — cdylib+rlib; serde, serde_json, thiserror deps — p16-c006 reconcile note: ships as `[lib] crate-type = ["rlib"]` only, not `["cdylib", "rlib"]` — a deliberate design choice (the crate's own doc comment: "contains no WIT calls of its own... compiles on any target including wasm32-wasip2"), arguably more correct for a library consumed by another crate, but diverges from the literal checkbox text
- [x] Create `crates/flint-skill/src/error.rs` — `SkillError` enum with thiserror
- [x] Create `crates/flint-skill/src/db.rs` — `Database::query(sql, params) -> Vec<Value>` — ships as `query(...) -> SkillResult<Vec<DbRow>>` (a JSON-backed typed wrapper, not raw `Vec<Value>`), functionally equivalent
- [x] Create `crates/flint-skill/src/llm.rs` — `Llm::complete(prompt, opts) -> String`; `Llm::embed(input, model) -> Vec<f32>` — ships as typed `CompletionResult`/`EmbeddingResult` wrappers (`.text: String`/`.vector: Vec<f32>`), functionally equivalent
- [x] Create `crates/flint-skill/src/kv.rs` — `Kv::get(key) -> Option<Vec<u8>>`; `Kv::set(key, val)` — matches exactly
- [x] Create `crates/flint-skill/src/identity.rs` — `Identity::claims() -> Value`; `Identity::origin_jwt() -> Option<String>`
- [x] Create `crates/flint-skill/src/secrets.rs` — `Secrets::get(name) -> SecretHandle`; `SecretHandle::reveal() -> String`
- [x] Create `crates/flint-skill/src/lib.rs` — public re-exports
- [x] Create `crates/flint-skill/tests/` — unit tests with mock bindings — `tests/integration.rs` with `MockDb`/`MockLlm`/`MockKv` etc., 8 tests passing
- [x] Create `crates/flint-skill/README.md` — quick-start guide
- [x] Add `flint-skill` to `[workspace.members]` in root `Cargo.toml`
- [x] `cargo check -p flint-skill` compiles
- [x] `cargo clippy -p flint-skill -- -D warnings` clean
- [x] `cargo test -p flint-skill` passes

<!-- p16-c006 reconcile (2026-07-13): verified every file/artifact exists and all gates pass (cargo check/clippy/test all clean, 10 unit + 2 doc + 8 integration tests green). A few API-shape details (crate-type, typed wrapper structs vs raw Value/String/Vec<f32>) diverge from the literal checkbox wording but are equivalent-or-better implementations, not missing work — noted inline rather than silently rubber-stamped. -->
