# p14-c002 Tasks — Kiln Guest Rust SDK

## Tasks

- [ ] Create `crates/flint-skill/Cargo.toml` — cdylib+rlib; serde, serde_json, thiserror deps
- [ ] Create `crates/flint-skill/src/error.rs` — `SkillError` enum with thiserror
- [ ] Create `crates/flint-skill/src/db.rs` — `Database::query(sql, params) -> Vec<Value>`
- [ ] Create `crates/flint-skill/src/llm.rs` — `Llm::complete(prompt, opts) -> String`; `Llm::embed(input, model) -> Vec<f32>`
- [ ] Create `crates/flint-skill/src/kv.rs` — `Kv::get(key) -> Option<Vec<u8>>`; `Kv::set(key, val)`
- [ ] Create `crates/flint-skill/src/identity.rs` — `Identity::claims() -> Value`; `Identity::origin_jwt() -> Option<String>`
- [ ] Create `crates/flint-skill/src/secrets.rs` — `Secrets::get(name) -> SecretHandle`; `SecretHandle::reveal() -> String`
- [ ] Create `crates/flint-skill/src/lib.rs` — public re-exports
- [ ] Create `crates/flint-skill/tests/` — unit tests with mock bindings
- [ ] Create `crates/flint-skill/README.md` — quick-start guide
- [ ] Add `flint-skill` to `[workspace.members]` in root `Cargo.toml`
- [ ] `cargo check -p flint-skill` compiles
- [ ] `cargo clippy -p flint-skill -- -D warnings` clean
- [ ] `cargo test -p flint-skill` passes
