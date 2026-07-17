# Flint Anon and Service Role Keys

Flint follows the Supabase-style dual-key model:

| Key | Variable | Safe in clients | Role | RLS |
|---|---|---:|---|---|
| Anon key | `FLINT_ANON_KEY` | Yes | `anon` | Applied |
| Service role key | `FLINT_SERVICE_ROLE_KEY` | No | `service_role` | Bypassed |

Generate local development keys:

```bash
forge keygen init --project my-project --env development --format env
```

This emits:

- `FLINT_JWT_SECRET`
- `FLINT_JWT_ALGORITHM`
- `FLINT_ANON_KEY`
- `FLINT_SERVICE_ROLE_KEY`
- `FLINT_PROJECT_ID`
- `FLINT_ENV`

`FLINT_SERVICE_ROLE_KEY` bypasses Postgres row-level security through the
`service_role` role. It must stay server-side only. `FLINT_ANON_KEY` is
publishable, but it is safe only when RLS policies are correct.

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
