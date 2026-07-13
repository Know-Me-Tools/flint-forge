# JWT Contract — Flint Forge × Flint Gate

**Status:** PINNED — derived from flint-gate source code by the flint-gate team  
**Version:** 1.1.0  
**Date:** 2026-07-13  
**Source:** `/Users/gqadonis/Projects/prometheus/flint-gate` (authenticated read)  
**Canonical fabric-wide spec:** [`flint-gate/docs/FLINT-KEYS.md`](../../../flint-gate/docs/FLINT-KEYS.md) —
this document is Forge's implementation-level elaboration of that spec, not a competing
source of truth. Where the two disagree, `FLINT-KEYS.md` wins and this file must be
corrected.  
**Resolves:** OQ-4 (claim shape), OQ-5 (service-identity token format)

**v1.1.0 correction:** §2.2 and §4 previously stated `HS256` as the default signing
algorithm. That was wrong even at v1.0.0 time — flint-gate's own inbound-verification
algorithm allowlist (`default_jwt_algorithms()`) and its JWKS endpoint have only ever
supported asymmetric algorithms, and this repo's verifier
(`forge-identity::verify_and_build`) has never accepted `HS*` at all. flint-gate's
*outbound minting* config still defaults to `HS256`
(`default_jwt_algorithm()` in `flint-gate-core/src/config/types.rs`, and
`config.example.yaml`) — that is a live misconfiguration trap, not a documentation
choice: a gate deployment left on that default mints tokens Forge can never verify.
Treat `signing_algorithm: RS256` (or `ES256`) as mandatory for any gate instance that
fronts Forge, until flint-gate closes that default/allowlist mismatch itself.

---

## 1. Inbound Token — User JWT (flint-gate verifies before Forge sees it)

Flint Gate **verifies** inbound Bearer tokens against a JWKS endpoint before forwarding any request to Forge (Quarry/Kiln). Postgres never sees unverified JWTs.

### 1.1 Verified claims flint-gate extracts

| Claim | Type | Required | Description |
|-------|------|----------|-------------|
| `sub` | `string` | Yes | User identifier (Kratos identity ID) |
| `iss` | `string` | Configurable | Issuer — validated if `jwt.issuer` is set in gate config |
| `aud` | `string` | Configurable | Audience — validated if `jwt.audience` is set |
| `iat` | `i64` | Yes | Issued-at (standard JWT) |
| `exp` | `i64` | Yes | Expiry (standard JWT) |
| `jti` | `string` | No | JWT ID (skipped in claim mapping) |
| `nbf` | `i64` | No | Not-before (skipped) |
| `auth_time` | `i64` | No | Authentication time (skipped) |

### 1.2 OIDC trait claims (mapped to `identity.traits`)

Forwarded verbatim into `request.jwt.claims` when present:

| Claim | Description |
|-------|-------------|
| `email` | User email |
| `email_verified` | Boolean |
| `name` | Display name |
| `given_name` | First name |
| `family_name` | Last name |
| `nickname` | Nickname |
| `preferred_username` | Username |
| `picture` | Avatar URL |
| `phone_number` | Phone |
| `locale` | Locale code |

### 1.3 Non-standard claims (mapped to `identity.metadata_public`)

Any claim NOT in the skip list or trait list lands in `metadata_public`, e.g.:
- `role` — **CRITICAL**: used by `auth.role()` RLS function
- `org_id`, `tenant_id` — tenant/org scoping
- `scope` — granted scopes
- Custom application claims

### 1.4 Kratos session path

When a Kratos session cookie/token is used, flint-gate calls `GET /sessions/whoami` and maps:

```
KratosSession.identity.id          → Identity.id (= sub for minted token)
KratosSession.identity.traits      → identity.traits
KratosSession.identity.metadata_public → identity.metadata_public
KratosSession.identity.schema_id   → identity.schema_id
KratosSession.id                   → identity.session_id
KratosSession.authenticator_assurance_level → identity.aal
```

---

## 2. Outbound Token — Service-Identity JWT (gate mints, Forge receives)

Flint Gate mints a **new** service-identity JWT via `JwtMinter` for upstream forwarding. This is the token Quarry/Kiln/extensions see in `request.headers.authorization`.

### 2.1 Default minted claim shape

```jsonc
{
  "iss": "flint-gate",          // configurable: jwt.issuer (default "flint-gate")
  "sub": "<identity.id>",       // Kratos identity ID or API key client_id
  "iat": 1751234567,            // current Unix timestamp
  "exp": 1751234867,            // iat + ttl (default 300 seconds)
  "jti": "<uuid-v4>",           // fresh UUID per request
  // --- merged from identity.traits (OIDC) ---
  "email": "user@example.com",  // if present in traits
  "name": "...",                // etc.
  // --- merged from additional_claims (per-route config) ---
  "scope": "chat",              // example from config.example.yaml
  "org_id": "..."               // example
}
```

### 2.2 Algorithm

**Asymmetric only — no exceptions.** Per the canonical `FLINT-KEYS.md`: RS256 by default,
matching flint-gate's own inbound-verification allowlist (`["RS256", "ES256"]`) and its
JWKS endpoint, which can only ever serve DB-sourced *public* keys.

- Default: **RS256** (`jwt.signing_key_path`, PEM-encoded private key)
- Also accepted: **ES256** / **ES384** (EC P-256 / P-384)
- DB-sourced key path: supported (DB key takes precedence over config)
- Algorithms Forge's verifier (`forge-identity::verify_and_build`) will accept:
  `RS256`, `RS384`, `RS512`, `ES256`, `ES384`
- **`HS256`/`HS384`/`HS512` are rejected outright** by Forge's verifier — and cannot work
  with JWKS-based verification in principle, since there is no way to publish a symmetric
  HMAC secret via a public JWKS endpoint. flint-gate's own `signing_algorithm` config
  currently *defaults* to `HS256` (see the v1.1.0 correction note above) — that default
  MUST be overridden to `RS256`/`ES256` for any deployment that fronts Forge.

### 2.3 Header injection into upstream

The minted JWT is injected as `Authorization: Bearer <token>` on the upstream request (overwriting the original user token). Additional template-rendered headers are also injected before the JWT:

```http
Authorization: Bearer <minted-jwt>
X-User-Id: {{ identity.id }}
X-User-Email: {{ identity.traits.email }}
X-Org-Id: {{ identity.metadata_public.org_id }}
X-Request-Id: <uuid>
```

---

## 3. What Postgres Sees (`SET LOCAL` context)

Quarry sets three `SET LOCAL` statements per transaction from the verified/minted token:

```sql
SET LOCAL ROLE authenticated;
SET LOCAL "request.jwt.claims" = '<json-of-claims>';
SET LOCAL "request.headers"    = '{"authorization":"Bearer <raw-service-jwt>"}';
```

### 3.1 Claims JSON shape written to `request.jwt.claims`

```jsonc
{
  "sub": "<identity.id>",
  "role": "<identity.metadata_public.role OR 'anon'>",
  "iss": "flint-gate",
  "iat": 1751234567,
  "exp": 1751234867,
  "jti": "<uuid>",
  "email": "user@example.com",   // from traits, if present
  "org_id": "...",               // from metadata_public, if present
  "tenant_id": "..."             // from metadata_public, if present
  // + any other additional_claims merged by the route hook
}
```

### 3.2 `auth.*` GUC accessors (from `ext-flint-auth/sql/flint_auth.sql`)

```sql
auth.jwt()       → jsonb -- current_setting('request.jwt.claims')::jsonb
auth.uid()       → text  -- auth.jwt()->>'sub'
auth.role()      → text  -- coalesce(auth.jwt()->>'role', 'anon')
auth.bearer()    → text  -- current_setting('request.headers')::json->>'authorization'
auth.tenant_id() → text  -- auth.jwt()->>'tenant_id'
```

**Critical:** `role` claim in the JWT is what `auth.role()` returns. Must be one of:
- `authenticated` — logged-in user
- `anon` — anonymous / no role claim
- `service_role` — internal service (see §4)

> **CRITICAL:** `auth.role()` returns `coalesce(jwt->>'role', 'anon')`. The `role` claim is
> NOT automatically included in minted JWTs by flint-gate — every production route hook
> MUST explicitly add `"role": "authenticated"` (or `"role": "service_role"`) to
> `additional_claims` in the flint-gate route configuration. Without this, all RLS
> role-gated policies will see `anon` for every request.

---

## 4. Service-Identity Token Format (OQ-5)

When flint-gate routes on behalf of internal services (hooks BGW, LLM worker, admin operations), the minted JWT carries:

```jsonc
{
  "iss": "flint-gate",
  "sub": "<service-name>",      // e.g. "flint-hooks-bgw", "flint-llm-worker"
  "role": "service_role",       // bypasses RLS in Postgres
  "iat": ...,
  "exp": ...,
  "jti": "<uuid>",
  "scope": "<service-scope>"    // e.g. "webhooks:dispatch", "llm:infer"
}
```

The `role: "service_role"` claim maps to the `service_role` Postgres role, which bypasses RLS. This is how `flint_hooks` and `flint_llm` operate on rows they need without being filtered by per-user policies.

**Algorithm for service tokens:** same constraint as user tokens (§2.2) — asymmetric only
(RS256 default, ES256/ES384 also accepted). flint-gate's deployment-wide
`signing_algorithm` config currently *defaults* to `HS256`; that default MUST be
overridden to `RS256`/`ES256` for any deployment fronting Forge, or service-identity
tokens for `flint_hooks`/`flint_llm` are unverifiable here for the same structural
reason user tokens are (§2.2).

**NEVER log:** The service token or its claims. Auth.bearer() must not appear in server logs.

---

## 5. Claim Propagation Contract (Phase 2+)

Phase 2 adds three additional GUCs via extended propagation:

```sql
SET LOCAL "app.jwt_claims"  = '<claims-json>';     -- Phase 2 addition
SET LOCAL "app.keto_subject" = '<keto-tuple-sub>'; -- Phase 3 addition
SET LOCAL "app.vault_key_id" = '<uuid-or-null>';   -- Phase 3 addition
```

These are set by `fdb-auth` crate during request transaction setup.

---

## 6. Security Constraints (Non-Negotiable)

- Postgres **never** verifies JWT signatures — flint-gate does that upstream
- `auth.bearer()` returns the raw Authorization header value — NEVER log it
- `role` claim controls which Postgres role is active — `service_role` bypasses RLS
- Claims JSON must not contain: DEK values, raw secrets, session tokens from vault
- `jti` is generated per-mint — no replay protection in Postgres (handled at gate layer)
- Leeway: 5 seconds default (`jwt.leeway_seconds`)

---

## 7. Integration Points for `ext-flint-auth`

The `auth.*` SQL functions (already in `crates/ext-flint-auth/sql/flint_auth.sql`) directly consume this contract:

```sql
-- What RLS policies write:
USING (auth.uid() = user_id)
USING (auth.role() = 'service_role')          -- bypass for service operations
USING (auth.jwt()->>'org_id' = org_id)        -- org scoping (no dedicated helper)
USING (auth.tenant_id() = tenant_id)          -- tenant scoping via dedicated helper
```

**Note on `role` vs `aud`:** Postgres RLS uses `auth.role()` → `request.jwt.claims.role`. This is NOT the JWT `aud` claim. The `aud` claim is validated at the gate but is not used in RLS policies.
