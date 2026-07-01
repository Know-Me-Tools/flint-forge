# p3-c005 — pg_graphql PG18: OQ-3 Resolution

## Change ID
`p3-c005-pg-graphql-pg18`

## Phase
`p3-graphql-hybrid-engine`

## Priority
P0 — Pre-kickoff gate; must complete before p3-c001 begins

## Problem Statement

OQ-3 (from `current-waypoint.json`) is unresolved: it is unknown whether
`supabase/pg_graphql` has a stable tagged release that supports Postgres 18.
`images/postgres18/Dockerfile` was built in Phase 1 (p0-c002) but the pg_graphql
version in that image has not been confirmed for PG18 compatibility.

Until this is resolved:
- `POST /graphql → graphql.resolve()` (p3-c001) cannot be coded
- The PG18 docker image may silently run the wrong pg_graphql or fail at runtime

## Scope

### In Scope
- Research `supabase/pg_graphql` GitHub releases for PG18 tagged release
- Document: version, install method, and any known PG18 caveats
- If no PG18 tagged release: identify the head commit that supports PG18, note it as "build from source at SHA"
- Write `docs/contracts/pg-graphql-version.md` with pinned version/SHA
- Verify or update `images/postgres18/Dockerfile` pg_graphql install step

### Out of Scope
- Modifying any Rust source code
- Running pg_graphql queries (that is p3-c001)

## Design

`docs/contracts/pg-graphql-version.md` will follow the format of `docs/contracts/jwt-contract.md`:

```markdown
# pg_graphql Version Contract

## Pinned Version
<!-- either: -->
pg_graphql tagged release: v<X.Y.Z>  (confirmed PG18 support)
<!-- or: -->
pg_graphql from source: SHA <commit-sha>  (PG18 tagged release not yet published)

## Install Method
<!-- e.g. extension install from PGXN, or build from source instructions -->

## Caveats
<!-- known PG18-specific issues -->

## Verified
<!-- date + verification method -->
```

## Security Contracts
- None (research-only change)

## Acceptance Criteria
- `docs/contracts/pg-graphql-version.md` exists with a concrete pinned version or SHA
- `images/postgres18/Dockerfile` pg_graphql step reflects the pinned version
- Assessment can confirm: "pg_graphql works with `SELECT graphql.resolve(...)` on PG18"
- Resolves OQ-3 in `current-waypoint.json`
