# p0-c002 — Postgres 18 image (the long pole)

## Why
Every Anvil capability and Quarry RLS behavior depends on a Postgres 18 image carrying the
right extensions and configuration. This is the highest-risk Phase 0 item.

## What
- Dockerfile: PG18 + pgvector + pg_net + pg_graphql + pgcrypto + Flint Anvil (`flint_auth`,
  `flint_hooks`, `flint_llm`).
- Config: `wal_level=logical`, `shared_preload_libraries='pg_net,ext_flint_llm'`.
- Boot init: `CREATE EXTENSION` for each.

## Contract
Container starts; `SELECT * FROM pg_available_extensions` lists all required extensions;
`SHOW wal_level` = logical. A boot assertion fails fast if any are missing.

## Open (confirm here — spec §8)
pgrx `pg18` feature availability; pg_graphql PG18 build; whether the prebuilt base already
ships pg_net/pgcrypto. If `wal_level=logical` is unavailable, fall back to a NOTIFY CDC source
in the fabric (transparent to Quarry).
