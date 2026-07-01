# p2-c001 — fdb-auth: JWT Verify → RlsContext

## Change ID
`p2-c001-fdb-auth`

## Phase
`p2-quarry-reflection-engine`

## Priority
P0 — MVP blocker

## Problem Statement

`forge_identity::verify_and_build()` is a `todo!()` stub. Every authenticated
REST request through Quarry fails at the boundary where it should verify the
flint-gate JWT, decode claims, and assemble the `RlsContext` that drives all
downstream `SET LOCAL` RLS propagation.

No request can carry correct RLS context until this is implemented.

## Scope

### In Scope
- Implement `forge_identity::verify_and_build(bearer: &str) -> Result<RlsContext, IdentityError>`
- JWKS fetch from `FLINT_GATE_JWKS_URL` with in-memory cache (`tokio::sync::OnceCell`)
- JWT signature verification via `jsonwebtoken` crate
- Claims decode into `RlsContext { role, claims_json, raw_bearer }`
- `role` claim coercion: absent `role` → `"anon"` (NOT an error)
- Environment variable validation at startup: `FLINT_GATE_ISSUER`, `FLINT_GATE_JWKS_URL`
- `thiserror`-based `IdentityError` with `#[non_exhaustive]` variants

### Out of Scope
- Keto relationship check (Phase 4)
- Cedar policy evaluation (Phase 4)
- Token refresh / session management (flint-gate's responsibility)
- Multi-issuer / multi-tenant JWKS rotation (Phase 6)

## Design

### JWKS Cache

```rust
// forge-identity/src/jwks.rs
use tokio::sync::OnceCell;
use jsonwebtoken::jwk::JwkSet;
use reqwest::Client;

static JWKS: OnceCell<JwkSet> = OnceCell::const_new();

pub async fn get_jwks(jwks_url: &str) -> Result<&'static JwkSet, IdentityError> {
    JWKS.get_or_try_init(|| async move {
        let set: JwkSet = Client::new()
            .get(jwks_url)
            .send()
            .await
            .map_err(IdentityError::JwksFetch)?
            .json()
            .await
            .map_err(IdentityError::JwksParse)?;
        Ok(set)
    })
    .await
}
```

### verify_and_build

```rust
// forge-identity/src/lib.rs
pub async fn verify_and_build(bearer: &str) -> Result<RlsContext, IdentityError> {
    let jwks_url = std::env::var("FLINT_GATE_JWKS_URL")
        .map_err(|_| IdentityError::MissingEnv("FLINT_GATE_JWKS_URL"))?;
    let issuer = std::env::var("FLINT_GATE_ISSUER")
        .map_err(|_| IdentityError::MissingEnv("FLINT_GATE_ISSUER"))?;

    let jwks = get_jwks(&jwks_url).await?;

    // Decode header to get kid
    let header = jsonwebtoken::decode_header(bearer)
        .map_err(|_| IdentityError::InvalidToken)?;
    let kid = header.kid.ok_or(IdentityError::MissingKid)?;

    // Find matching JWK
    let jwk = jwks.find(&kid).ok_or(IdentityError::UnknownKid(kid))?;

    // Validate signature + standard claims
    let mut validation = jsonwebtoken::Validation::new(header.alg);
    validation.set_issuer(&[&issuer]);
    validation.set_audience(&["flint-api"]);

    let decoded = jsonwebtoken::decode::<serde_json::Value>(
        bearer,
        &jsonwebtoken::DecodingKey::from_jwk(jwk)
            .map_err(|_| IdentityError::InvalidToken)?,
        &validation,
    )
    .map_err(|_| IdentityError::InvalidToken)?;

    // CRITICAL: role claim NOT auto-included in minted JWTs.
    // Absent role → coerce to "anon". This is intentional, NOT an error.
    // See docs/contracts/jwt-contract.md §CRITICAL block.
    let role = decoded.claims
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("anon")
        .to_string();

    let claims_json = serde_json::to_string(&decoded.claims)
        .map_err(|_| IdentityError::ClaimsSerialize)?;

    Ok(RlsContext {
        role,
        claims_json,
        raw_bearer: bearer.to_string(),
    })
}
```

### IdentityError

```rust
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("JWKS fetch failed")]
    JwksFetch(#[source] reqwest::Error),
    #[error("JWKS parse failed")]
    JwksParse(#[source] reqwest::Error),
    #[error("invalid or expired token")]
    InvalidToken,
    #[error("token missing kid header")]
    MissingKid,
    #[error("unknown kid: {0}")]
    UnknownKid(String),
    #[error("claims serialization failed")]
    ClaimsSerialize,
    #[error("missing required env var: {0}")]
    MissingEnv(&'static str),
}
```

## Security Contracts (NON-NEGOTIABLE)

1. **Never log `bearer`, `claims_json`, or decoded claim values** — only error codes
2. **`role` absent → `"anon"`** — this is a defined behavior, not a security hole; RLS
   policies treat `anon` appropriately. Do not error on missing role.
3. **JWKS cached in `OnceCell`** — one fetch per process lifetime; rotation handled by
   process restart (acceptable for Phase 2; Phase 6 adds TTL rotation)
4. **`fdb-auth` is a thin wrapper** — all JWT logic lives in `forge-identity`; `fdb-auth`
   calls `forge_identity::verify_and_build()` and passes `RlsContext` to the pool layer

## Dependencies to Add

### `forge-identity/Cargo.toml`
```toml
jsonwebtoken = { workspace = true }
reqwest = { workspace = true, features = ["json"] }
tokio = { workspace = true, features = ["sync"] }
serde_json = { workspace = true }
thiserror = { workspace = true }
```

### `[workspace.dependencies]` in root `Cargo.toml`
```toml
jsonwebtoken = "9"
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
```

## Files Affected

| File | Change |
|---|---|
| `crates/forge-identity/src/lib.rs` | Replace `todo!()` with full implementation |
| `crates/forge-identity/src/jwks.rs` | NEW — JWKS cache |
| `crates/forge-identity/src/error.rs` | NEW — `IdentityError` |
| `crates/forge-identity/Cargo.toml` | Add `jsonwebtoken`, `reqwest`, `tokio/sync` |
| `Cargo.toml` | Add `jsonwebtoken`, `reqwest` to `[workspace.dependencies]` |
| `crates/fdb-auth/src/lib.rs` | No signature change; now calls async `verify_and_build` |

## Gate Criteria

- `cargo check --workspace` passes (no new warnings)
- `cargo clippy --workspace -- -D warnings` passes
- Unit tests pass: `cargo test -p forge-identity`
- Tests cover: valid JWT → correct `RlsContext`, missing `role` → `"anon"`, expired JWT →
  `InvalidToken`, unknown `kid` → `UnknownKid`, `FLINT_GATE_JWKS_URL` absent → `MissingEnv`
- No JWT payload values appear in any `tracing` span or log line
