# Flint Anon and Service Role Keys

Flint follows the Supabase-style dual-key model:

| Key | Variable | Safe in clients | Role | RLS |
|---|---|---:|---|---|
| Anon key | `FLINT_ANON_KEY` | Yes | `anon` | Applied |
| Service role key | `FLINT_SERVICE_ROLE_KEY` | No | `service_role` | Bypassed |

> **Reality check (p17-c001).** Two corrections to earlier revisions of this
> document:
>
> 1. **No code in this repository reads `FLINT_ANON_KEY` or
>    `FLINT_SERVICE_ROLE_KEY`.** They are *client-held credentials* — bearer
>    tokens a caller presents in the `Authorization` header — not Forge
>    configuration. Grep the workspace: zero readers.
> 2. **`forge keygen init` does not exist.** `forge-cli` has no `keygen`
>    subcommand (the spec that proposed it, `FLINT_ANON_SERVICE_ROLE_KEYS_SPEC.md`
>    §3.1, was implemented elsewhere — see below). The working generator is
>    `sansaba-workspace/infra/scripts/generate-keys.mjs`, which emits RS256
>    keys with a `kid` header, `role: "anon"` / `role: "service_role"` claims,
>    `iss`/`aud` = `flint-forge`, a 10-year expiry, the public half as
>    `infra/keys/jwks.json` (serve it at `FLINT_GATE_JWKS_URL`), and both
>    tokens into a git-ignored `.env.keys`. Re-running it ROTATES: prior
>    tokens die once the served JWKS refreshes.

Generate working keys (from the sansaba-workspace checkout):

```bash
node infra/scripts/generate-keys.mjs
```

`FLINT_SERVICE_ROLE_KEY` bypasses Postgres row-level security through the
`service_role` role (a real `BYPASSRLS` attribute since migration `0014`). It
must stay server-side only. `FLINT_ANON_KEY` is publishable, but it is safe
only when RLS policies are correct. Because both keys are 10-year, **expiry is
not a security control — rotation is the revocation path.**

## Roles

`ext-flint-auth` installs:

- `anon`
- `authenticated`
- `agent`
- `service_role`
- `authenticator`

`authenticator` is the bridge role used by pooled database connections. The JWT
claim `role` determines the request role, while helper functions read the same
claim set through `request.jwt.claims`:

- `auth.uid()`
- `auth.role()`
- `auth.tenant_id()`
- `auth.agent_id()`
- `auth.workflow_id()`
- `auth.principal_type()`
- `auth.is_service_role()`

## Token Minting

Development/local token minting remains available:

```bash
forge token mint \
  --secret "$FLINT_JWT_SECRET" \
  --role agent \
  --principal-type Agent \
  --subject user-uuid \
  --agent-id agent-uuid \
  --workflow-id workflow-uuid \
  --scope "read:documents mcp:tool:read"
```

Production signing authority belongs in `flint-gate`; `forge-cli` is the local
initialization and operator wrapper.

> **Note:** `forge token mint` signs with HS256 using `FLINT_JWT_SECRET`.
> `fdb-gateway`'s bearer verification (`forge-identity::verify_and_build`)
> only accepts JWKS-verified RS256/RS384/RS512/ES256/ES384 tokens and never
> reads `FLINT_JWT_SECRET`. A token minted this way will not authenticate
> against `fdb-gateway` in any environment as the code stands today. See
> [`docs/runbook.md §2.2`](runbook.md) for the real inbound-auth requirements.
