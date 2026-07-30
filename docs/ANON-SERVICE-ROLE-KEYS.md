# Anon and Service-Role Keys — Creation, Management, and Use

Flint follows the Supabase-style dual-key model:

| Key | Variable | Safe in clients | JWT `role` claim | RLS |
|---|---|---:|---|---|
| Anon key | `FLINT_ANON_KEY` | Yes | `anon` | Applied |
| Service role key | `FLINT_SERVICE_ROLE_KEY` | **No** | `service_role` | Bypassed (real `BYPASSRLS`, migration `0014`) |

Both are long-lived (10-year) RS256 JWTs carrying `role`, `sub`,
`principal_type`, `iss`/`aud` = `flint-forge`, and a `kid` header. They are
**client-held credentials** presented in the `Authorization` header — no code
in this repository reads them as configuration (the only in-repo readers are
environment-gated integration tests that use them *as* the credentials under
test).

## Creating keys — the flint-gate minting utility

Keys are minted by **`flint-gate`'s `scripts/generate-forge-keys.mjs`**
(promoted from the Sansaba workspace per FFS-001 §11.6 so every Forge
consumer uses one utility instead of copying key-minting code between repos —
which is how private keys end up in the wrong repository).

```bash
cd flint-gate
FLINT_PROJECT=acme FLINT_ENV=production node scripts/generate-forge-keys.mjs
```

Outputs (all git-ignored, private material `0600`):

| File | Content | Handling |
|---|---|---|
| `keys/jwt-private.pem` | RS256 signing key | **NEVER commit. NEVER serve.** |
| `keys/jwks.json` | Public half (JWKS with the `kid`) | Serve to verifiers — this is the file behind `FLINT_GATE_JWKS_URL` |
| `.env.keys` | `FLINT_ANON_KEY`, `FLINT_SERVICE_ROLE_KEY`, plus the matching `FLINT_GATE_*` values | Distribute the anon key freely; the service key to trusted servers only |

Full utility documentation (flags, claims, JWKS serving options, examples):
the **Forge Keys** page in flint-gate's Docusaurus docs
(`flint-gate/docs/docs/forge-keys.md`).

Why RS256 and not the HS256 that flint-gate's runtime defaults to: Forge's
verifier (`forge-identity::verify_and_build`) is JWKS-only — it requires a
`kid` header and accepts RS256/RS384/RS512/ES256/ES384. A shared secret has
no public half to publish, so an HS256 token can never satisfy Forge's
verification path.

## Wiring Forge to verify the keys

```bash
# Where the public JWKS is served (see "Serving the JWKS" below)
FLINT_GATE_JWKS_URL=https://keys.example.com/jwks.json
# Must equal the iss/aud the utility mints — both are `flint-forge`
FLINT_GATE_ISSUER=flint-forge
FLINT_GATE_AUDIENCE=flint-forge
# Optional: production (default) fails closed when AUDIENCE is unset
# FLINT_GATE_MODE=production
# Optional: JWKS cache TTL — see "Rotation" for why you may want it lower
# FLINT_GATE_JWKS_TTL_SECS=600
```

Every request's bearer is signature-verified against the JWKS (Postgres never
verifies JWTs); the decoded `role` claim becomes the `SET LOCAL ROLE` for the
request transaction, the full claim set lands in `request.jwt.claims` for RLS
policies and the `auth.*` helpers, and the raw bearer is forwarded for
outbound use by `flint_hooks`/`flint_llm` via `auth.bearer()`.

### Serving the JWKS

Anything that serves the static `jwks.json` over HTTP works:

```bash
# Local development
npx serve flint-gate/keys          # or: python3 -m http.server --directory flint-gate/keys 8917
export FLINT_GATE_JWKS_URL=http://127.0.0.1:3000/jwks.json
```

In production, serve it from any static origin your gateways can reach
(object storage behind a CDN is fine — it is public material). Keep the URL
stable; rotation replaces the file's *contents*.

## Using the keys

**Anon key — publishable, RLS-gated.** Safe in browsers and mobile apps
*only because* RLS policies constrain every row it can touch. It is not a
secret; it is a scoped identity.

```bash
# Read via the reflected REST surface as the anon role
curl -sS "$FORGE_URL/public/articles?status=eq.published" \
  -H "Authorization: Bearer $FLINT_ANON_KEY"
```

**Service-role key — server-side only, bypasses all RLS.** Treat it like a
database superuser password. Uses: backend jobs, admin tooling, Kiln's
`/admin/functions` control plane, and the entire
[Schema Provisioning API](api/schema-provisioning.md) (`/schema/v1`), which
refuses any other role with `403`.

```bash
# Plan + apply a declared table (see the provisioning API docs for the full flow)
curl -sS -X POST "$FORGE_URL/schema/v1/plan" \
  -H "Authorization: Bearer $FLINT_SERVICE_ROLE_KEY" \
  -H "content-type: application/json" -d @entities.spec.json
```

**End-user tokens are neither of these.** Per-user JWTs (real `sub`, tenant
claims, short expiry) are minted by flint-gate's runtime minter on
authentication flows and ride the same verification pipeline. The two
long-lived keys are *application* credentials in the Supabase sense.

## Rotation — the revocation path

These keys carry no user data and a 10-year expiry, so **expiry is not a
security control; rotation is.** Re-running the utility regenerates the
keypair and both tokens, and rewrites `jwks.json` with a new `kid` — once
the served JWKS is refreshed, tokens signed by the old key fail verification
(`kid` no longer resolvable, refetch-on-unknown-kid confirms promptly).

**Know the latency window.** Forge's JWKS cache is process-global with a
`FLINT_GATE_JWKS_TTL_SECS` TTL (default 600s), so a rotated-out key keeps
verifying on a warm gateway for up to that long after the JWKS changes. For
incident-grade revocation:

```bash
# 1. Preserve the old token for the post-rotation verification test
cp .env.keys .env.keys.pre-rotation
# 2. Rotate
FLINT_PROJECT=acme FLINT_ENV=production node scripts/generate-forge-keys.mjs
# 3. Publish the new jwks.json to the FLINT_GATE_JWKS_URL origin
# 4. RESTART every gateway (or run with a low FLINT_GATE_JWKS_TTL_SECS)
# 5. Distribute the new keys to services; verify the old key is dead:
FLINT_OLD_SERVICE_ROLE_KEY=$(source .env.keys.pre-rotation; echo $FLINT_SERVICE_ROLE_KEY) \
  cargo test -p fdb-gateway --test phase_boundary_e2e rotated_out_service_role_key_is_refused
```

The rotation mechanics themselves are covered continuously by
`crates/fdb-gateway/tests/rotation_revocation.rs`, which simulates a
rotation against a scratch JWKS in-process.

## Best practices

- **Never** commit `jwt-private.pem` or `.env.keys`; never log a bearer; the
  service key never reaches a browser, mobile app, or public repo.
- The anon key is only as safe as your RLS. Provision tables
  `tenantScoped: true` (the [provisioning API](api/schema-provisioning.md)
  generates the policies) and remember: a table without RLS is not exposed
  by Forge's reflection surfaces at all.
- One key set per project × environment (`FLINT_PROJECT`/`FLINT_ENV` shape
  the `kid` and `jti`), so a staging leak never touches production.
- Rotate on any suspicion of exposure, on personnel changes with key access,
  and on a calendar cadence of your choosing — and always follow a rotation
  with gateway restarts (see the latency window above).
- Keep the service key's standing power bounded: the provisioning API
  additionally requires per-namespace operator grants to
  `flint_provisioner` ([runbook §14](runbook.md)), so even a leaked service
  key cannot create tables outside the allowlist.

## Postgres roles behind the claims

`ext-flint-auth` installs `anon`, `authenticated`, `agent`, `service_role`,
and `authenticator` (the pooled-connection bridge role). The JWT `role`
claim selects the request role via `SET LOCAL ROLE`; the same claim set is
readable in SQL through `auth.uid()`, `auth.role()`, `auth.tenant_id()`,
`auth.agent_id()`, `auth.workflow_id()`, `auth.principal_type()`, and
`auth.is_service_role()`.

## Relationship to flint-gate's runtime minter (and `forge token mint`)

- **flint-gate's runtime JWT minter** (`flint_gate_core::auth::jwt_mint`)
  issues short-TTL *session* tokens during auth flows, and Gate forwards
  identity downstream via trusted `X-Flint-*` headers
  (`docs/FLINT-KEYS.md` in flint-gate). That is a different mechanism from
  the two long-lived keys, which are verified directly from the token's
  `role` claim by `forge-identity`.
- **`forge token mint`** (forge-cli) signs HS256 with `FLINT_JWT_SECRET` —
  useful for components that accept it, but **it cannot authenticate against
  `fdb-gateway`**, whose verification is JWKS/asymmetric-only and never
  reads `FLINT_JWT_SECRET`. Use the flint-gate utility above for anything
  that must pass Forge's bearer verification.
