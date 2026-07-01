# Tasks — p2-c001-fdb-auth

## Change
JWT Verify → RlsContext via JWKS cache in `forge-identity`

## Status: PENDING

---

## Task List

### T1 — Add workspace dependencies
- [ ] Add `jsonwebtoken = "9"` to `[workspace.dependencies]` in root `Cargo.toml`
- [ ] Add `reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }` to workspace deps
- [ ] Verify `cargo check --workspace` still passes after dep additions

### T2 — Create `forge-identity/src/error.rs`
- [ ] Define `IdentityError` enum with `thiserror` and `#[non_exhaustive]`
- [ ] Variants: `JwksFetch`, `JwksParse`, `InvalidToken`, `MissingKid`, `UnknownKid(String)`, `ClaimsSerialize`, `MissingEnv(&'static str)`
- [ ] Export from `forge-identity/src/lib.rs`

### T3 — Create `forge-identity/src/jwks.rs`
- [ ] Define `static JWKS: OnceCell<JwkSet>` using `tokio::sync::OnceCell`
- [ ] Implement `get_jwks(jwks_url: &str) -> Result<&'static JwkSet, IdentityError>`
- [ ] Use `reqwest::Client` to fetch and deserialize the JWKS JSON
- [ ] SECURITY: do not log the JWKS URL response body

### T4 — Implement `forge_identity::verify_and_build()`
- [ ] Read `FLINT_GATE_JWKS_URL` and `FLINT_GATE_ISSUER` from env; return `MissingEnv` if absent
- [ ] Call `get_jwks()` to load JWKS
- [ ] Decode JWT header to get `kid`; return `MissingKid` if absent
- [ ] Find matching JWK by `kid`; return `UnknownKid(kid)` if not found
- [ ] Build `jsonwebtoken::Validation` with issuer + audience (`"flint-api"`)
- [ ] Call `jsonwebtoken::decode()` with `DecodingKey::from_jwk()`; return `InvalidToken` on failure
- [ ] Extract `role` from claims; coerce absent/null `role` to `"anon"` (NOT an error)
- [ ] Serialize claims to `claims_json`; populate `RlsContext { role, claims_json, raw_bearer }`
- [ ] SECURITY: no claim values in `tracing` spans; only error codes

### T5 — Update `forge-identity/Cargo.toml`
- [ ] Add `jsonwebtoken`, `reqwest`, `tokio/sync`, `serde_json`, `thiserror` deps

### T6 — Unit tests in `forge-identity/tests/`
- [ ] `test_valid_jwt_returns_correct_rls_context` — use a test JWK pair
- [ ] `test_missing_role_coerces_to_anon`
- [ ] `test_expired_jwt_returns_invalid_token`
- [ ] `test_unknown_kid_returns_error`
- [ ] `test_missing_jwks_url_env_returns_missing_env`
- [ ] Assert no JWT payload values appear in any log output (tracing subscriber capture)

### T7 — Verify `fdb-auth` calls async `verify_and_build`
- [ ] Update `fdb-auth/src/lib.rs` to `await` the now-async `verify_and_build()`
- [ ] Ensure `fdb-auth/Cargo.toml` has `tokio` dep if not already present

### T8 — Final verification
- [ ] `cargo test -p forge-identity` — all 6 tests pass
- [ ] `cargo clippy --workspace -- -D warnings` — no warnings
- [ ] `cargo check --workspace` — clean build
