# Goals — anon_and_service_role_keys

## Phase Summary

Implement Supabase-style Flint project keys across the fabric: a publishable
`anon` key, a secret `service_role` key, and an agent-aware role model that works
through `flint-forge`, `flint-gate`, and `flint-realtime-fabric`.

Seeded from: user directive + attached "Prometheus Flint: Anon & Service Role
Key Generation" specification.

---

## Changes (5 planned)

### P0 — Key material and database role foundation

- **ark-c001 — Forge keygen and token claims:**
  Add `forge keygen init` / `rotate`, emit `FLINT_ANON_KEY` and
  `FLINT_SERVICE_ROLE_KEY`, and extend `forge token mint` for `anon`,
  `authenticated`, `agent`, and `service_role` claims.

- **ark-c002 — Forge auth SQL role model:**
  Extend `ext-flint-auth` with `agent`, `authenticator`, statement timeouts,
  agent claim helpers, `auth.is_service_role()`, and an `auth.api_keys` table
  for opaque-key compatibility.

### P1 — Gate and Realtime integration

- **ark-c003 — Gate API-key roles and trusted headers:**
  Preserve the role/principal type from key records, reject browser-presented
  secret keys, and expose Flint trusted header helpers for upstream forwarding.

- **ark-c004 — Realtime principal metadata propagation:**
  Parse `role`, `principal_type`, `agent_id`, `workflow_id`, and `scope` from
  verified JWTs so Realtime can apply the same key/agent boundaries as Forge.

### P2 — Cross-repo docs and verification

- **ark-c005 — Cross-project configuration docs:**
  Document the key contract and environment variables in the three sibling
  projects, including which key is client-safe and which one bypasses RLS.

---

## Phase Complete When

- [ ] `forge keygen init --project <id>` emits usable anon and service-role JWTs.
- [ ] `forge token mint` can mint `anon`, `authenticated`, `agent`, and
      `service_role` tokens with principal metadata.
- [ ] `ext-flint-auth` installs the four-role model and agent helper functions.
- [ ] `flint-gate` validates key records with role/principal metadata and
      produces trusted Flint headers for Forge/Realtime.
- [ ] `flint-realtime-fabric` preserves role/principal metadata from verified JWTs.
- [ ] Docs in `flint-forge`, `flint-gate`, and `flint-realtime-fabric` describe
      the shared contract.

---

## Constraints

- `flint-gate` remains the canonical signing authority for production token
  minting. `forge-cli` local signing is for initialization and development.
- `service_role` must never be treated as browser-safe.
- `anon` remains RLS-governed; access safety depends on Postgres policies.
- Agent tokens are RLS-governed and must carry agent/workflow metadata when
  scoped to autonomous workloads.
