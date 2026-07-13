# p16-c005 — Auth Hardening (JWKS Refresh + Mandatory Audience)

**Phase:** 16 — Production Remediation
**Priority:** P1
**Depends on:** none

## What this change delivers

- JWKS keys refresh on a TTL or on unknown-`kid`, so upstream key rotation
  does not require a `fdb-gateway` process restart.
- Audience validation is mandatory in production mode — a missing
  `FLINT_GATE_AUDIENCE` fails closed instead of silently skipping the check.

## Problem

`forge-identity`'s JWKS cache is a process-lifetime `OnceLock` with no TTL or
refresh (`jwks.rs:3-4, 13, 28-30`). When the upstream signing key rotates,
every token signed with the new key fails to verify (`kid` not found) until
the gateway process restarts — an availability incident triggered purely by
routine key rotation.

Audience validation only runs `set_issuer`/`set_audience` when
`FLINT_GATE_AUDIENCE` is set (`lib.rs:105-109`); if the env var is absent (e.g.
a misconfigured deployment), audience checking is silently skipped rather than
failing closed.

## Design

### JWKS refresh

Replace the bare `OnceLock` with a cache that supports either:
- **TTL-based refresh**: re-fetch JWKS every N minutes (configurable,
  reasonable default e.g. 10-15 min) in a background task or lazily on next
  access past TTL.
- **Refetch-on-unknown-`kid`**: on a verification failure due to unknown
  `kid`, refetch JWKS once and retry verification before failing — bounded to
  avoid a refetch storm (rate-limit refetches, e.g. max once per few seconds).

Prefer combining both: TTL as the steady-state refresh, refetch-on-unknown-kid
as the fast path for an unplanned rotation.

### Mandatory audience

Add an explicit `FLINT_GATE_MODE=production|development` (or reuse an existing
env convention if one exists in the codebase) that, in production mode, treats
a missing `FLINT_GATE_AUDIENCE` as a startup configuration error rather than a
silently-skipped check. Development mode may keep the current lenient
behavior for local iteration.

## Verification (gate)

- Test: simulate a JWKS key rotation (swap the mock JWKS response mid-test);
  assert a token signed with the new key verifies without a process restart.
- Test: refetch-on-unknown-`kid` path is exercised and rate-limited (doesn't
  refetch on every single request during an actual attack/misconfiguration).
- Test: in production mode, missing `FLINT_GATE_AUDIENCE` causes a startup
  failure (or a hard-deny on every request, whichever design is chosen) rather
  than silent audience-check skip.
- Test: in production mode, a token with a wrong audience is rejected.
