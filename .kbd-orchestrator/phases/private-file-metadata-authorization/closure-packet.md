# Closure packet — private-file-metadata-authorization

Disposition: **CLOSED** (budget-exhausted). This packet is a derived view of
the bound contract (sha256:514cbc7eee9c20b8c9475d58f6e96591ef24cc54e78a549ad7c1b27f84f99b07) and
`boundary-results.json` (sha256:16ad3f4ff61352a1ed1d8de85ed0ca2db458209e4a18b9f6534fca3f7c2731d9); the canonical KBD disposition is authoritative.

## What works, and for whom

- FILE-1 — New metadata at revision 1 and amended metadata at revision 2 (for Granted synthetic owner)
- FILE-2 — 403 metadata_denied for valid insert and update, missing inserted row and unchanged seeded row (for Ungranted owner)
- FILE-3 — Other-tenant and same-tenant other-subject cannot change or see the original row under RLS or persist a foreign-owned insert (for Granted foreign identities)
- FILE-4 — Type PUT returns 403 and independent lookup finds no new type (for Granted record owner)
- FILE-5 — Tenant-scoped grant is denied on a valid own-namespace write outside its approved tenant, or reviewed identity source explicitly authorizes globally scoped subjects (for Owner with the same subject in another tenant)

## Under which conditions

- environment: Isolated Forge gateway in rls mode and disposable PostgreSQL with migration 0021
- FILE-1 real collaborator: JWT middleware
- FILE-1 real collaborator: CedarPolicyEngine with reviewed policy
- FILE-1 real collaborator: PostgreSQL authenticated role and forced RLS
- FILE-2 real collaborator: JWT middleware
- FILE-2 real collaborator: CedarPolicyEngine with reviewed policy
- FILE-2 real collaborator: PostgreSQL authenticated role and forced RLS
- FILE-3 real collaborator: JWT middleware
- FILE-3 real collaborator: CedarPolicyEngine with reviewed policy
- FILE-3 real collaborator: PostgreSQL authenticated role and forced RLS
- FILE-4 real collaborator: JWT middleware
- FILE-4 real collaborator: CedarPolicyEngine with reviewed policy
- FILE-4 real collaborator: PostgreSQL authenticated role and forced RLS
- FILE-5 real collaborator: JWT middleware
- FILE-5 real collaborator: CedarPolicyEngine with reviewed policy
- FILE-5 real collaborator: PostgreSQL authenticated role and forced RLS
- FILE-1 SUBSTITUTED boundary (not a real claim): Synthetic issuer and fixture owners
- FILE-2 SUBSTITUTED boundary (not a real claim): Synthetic issuer and fixture owners
- FILE-3 SUBSTITUTED boundary (not a real claim): Synthetic issuer and fixture owners
- FILE-4 SUBSTITUTED boundary (not a real claim): Synthetic issuer and fixture owners
- FILE-5 SUBSTITUTED boundary (not a real claim): Synthetic issuer and fixture owners

## Limits

- FILE-1: Console session exchange and live deployment are outside this fixture.
- FILE-2: Console session exchange and live deployment are outside this fixture.
- FILE-3: Console session exchange and live deployment are outside this fixture.
- FILE-4: Console session exchange and live deployment are outside this fixture.
- FILE-5: Console session exchange and live deployment are outside this fixture.
- unknown: Approved binding revision and entitlement semantics
- unknown: Reviewed policy revision
- unknown: Runtime acceptance and deployment state

## What remains

- nothing: every scenario passed

## Evidence references

- candidate: not recorded in boundary-results.json
- venue: not recorded

References are reported as recorded. They change no verdict and no disposition.

## Pre-authorization runner artifacts

- no runnerOutputGlobs declared in the contract; scan not possible

This scan is passive evidence (VLBV 04, 10): it reports and never blocks.

## Closure states

| State | Value |
|---|---|
| Implementation complete | false |
| Behavior accepted | true |
| Release eligible | true |
| Release authorized | false (a separate owner decision) |

Method: VLBV.
Executed by: not recorded in boundary-results.json.
