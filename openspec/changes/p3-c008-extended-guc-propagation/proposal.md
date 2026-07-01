# p3-c008 — Extended GUC Propagation

## Change ID
`p3-c008-extended-guc-propagation`

## Phase
`p3-graphql-hybrid-engine`

## Priority
P0 — Required before p3-c001; also closes Phase 2 security debt

## Problem Statement

`PgBackend::acquire()` currently sets three `SET LOCAL` statements per request
transaction:
1. `SET LOCAL ROLE $role`
2. `SET LOCAL "request.jwt.claims" = $claims_json`
3. `SET LOCAL "request.headers" = $headers_json`

Phase 3 requires three additional GUC values that `flint_auth` extensions and
Keto inline checks read:
- `app.jwt_claims` — redundant alias used by some pgrx extensions that read
  `current_setting('app.jwt_claims', true)` (different namespace from `request.jwt.claims`)
- `app.keto_subject` — the subject string passed to `flint_meta.check_permission()`
  SQL calls from within the subscription delivery path
- `app.vault_key_id` — Vault key selector per request, consumed by `flint_vault`
  when a table has encrypted columns

Additionally, `forge-identity::verify_and_build()` is missing
`#[instrument(skip(bearer))]` — the raw bearer token may appear in tracing
spans, violating the JWT security contract (CLAUDE.md: "Never log JWT payloads,
claims, relation tuples, or tenant identifiers").

## Scope

### In Scope
- Add `keto_subject: Option<String>` and `vault_key_id: Option<String>` to `RlsContext`
- Populate these fields in `verify_and_build()` from decoded `Claims`:
  - `keto_subject` = `claims.sub` (always present per jwt-contract.md)
  - `vault_key_id` = `claims.extra.get("vault_key_id")` (optional JWT claim)
- Add `#[instrument(skip(bearer))]` to `verify_and_build()` (closes Phase 2 security debt)
- Add 3 additional `SET LOCAL` statements inside `PgBackend::acquire()`'s existing `BEGIN` transaction:
  ```sql
  SET LOCAL "app.jwt_claims" = $claims_json;
  SET LOCAL "app.keto_subject" = $keto_subject;   -- or empty string if absent
  SET LOCAL "app.vault_key_id" = $vault_key_id;   -- or empty string if absent
  ```
- Unit tests for the new `RlsContext` fields

### Out of Scope
- Changing `fdb-auth::rls_from_bearer()` API signature (callers pass bearer, get RlsContext — unchanged)
- Any changes to pgrx extensions (the SQL-side GUC readers already exist)

## Design

### RlsContext (forge-identity/src/lib.rs)

```rust
#[derive(Debug, Clone)]
pub struct RlsContext {
    pub role: String,
    pub claims_json: String,
    pub raw_bearer: String,
    // Phase 3 additions:
    pub keto_subject: String,          // always present (= sub claim); empty string for anon
    pub vault_key_id: Option<String>,  // optional JWT claim "vault_key_id"
}
```

### verify_and_build() (forge-identity/src/lib.rs)

```rust
#[instrument(skip(bearer), err)]   // <-- closes Phase 2 security gap
pub async fn verify_and_build(bearer: &str) -> Result<RlsContext, IdentityError> {
    // ... existing verification ...
    let keto_subject = claims.sub.clone();
    let vault_key_id = claims.extra
        .get("vault_key_id")
        .and_then(|v| v.as_str())
        .map(String::from);
    Ok(RlsContext {
        role,
        claims_json,
        raw_bearer: bearer.to_string(),
        keto_subject,
        vault_key_id,
    })
}
```

### PgBackend::acquire() — 3 additional SET LOCAL (fdb-postgres/src/lib.rs)

Inside the existing `BEGIN` transaction block, after the current 3 SET LOCAL calls:

```rust
let keto_subject = &rls.keto_subject;
object.execute(r#"SET LOCAL "app.jwt_claims" = $1"#, &[&rls.claims_json]).await
    .map_err(|e| PgError::SetLocal(format!(r#"SET LOCAL "app.jwt_claims": {e}"#)))?;
object.execute(r#"SET LOCAL "app.keto_subject" = $1"#, &[keto_subject]).await
    .map_err(|e| PgError::SetLocal(format!(r#"SET LOCAL "app.keto_subject": {e}"#)))?;
let vault_key_id = rls.vault_key_id.as_deref().unwrap_or("");
object.execute(r#"SET LOCAL "app.vault_key_id" = $1"#, &[&vault_key_id]).await
    .map_err(|e| PgError::SetLocal(format!(r#"SET LOCAL "app.vault_key_id": {e}"#)))?;
```

## Security Contracts
- ALL 6 `SET LOCAL` statements MUST be inside the same `BEGIN` transaction
- `raw_bearer` MUST remain excluded from tracing spans (`#[instrument(skip(bearer))]`)
- `keto_subject` and `claims_json` MUST NOT appear in log output (these are PII)
- `vault_key_id` is not secret but MUST NOT appear in spans either

## Acceptance Criteria
- `#[instrument(skip(bearer), err)]` present on `verify_and_build()`
- `RlsContext` has `keto_subject: String` and `vault_key_id: Option<String>`
- `PgBackend::acquire()` issues 6 `SET LOCAL` statements inside one `BEGIN` transaction
- Unit test `test_rls_context_keto_subject_from_claims` passes
- Unit test `test_rls_context_vault_key_id_from_extra_claim` passes
- `cargo check --workspace` clean; clippy pedantic passes
