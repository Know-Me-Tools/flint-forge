# p35-c004 — Make DB-integration tests gating + add missing coverage

## Change ID
`p35-c004-db-integration-tests`

## Phase
`p3.5-ci-postgres-hardening`

## Goal Mapping
**G2** — the live-PG tests run in CI; add the missing embedding + PgRest DB coverage.

## Depends on
`p35-c003` (CI DB service + `DATABASE_URL` must exist).

## Problem
The DB-backed tests are effectively dark: some are `#[ignore]`d (blanket-skipped even
when a DB is present), and the real-time embedding path has no DB-backed test at all —
only SQL-shape unit tests.

## Scope
- **Gate on `DATABASE_URL` presence, not `#[ignore]`**: convert so tests run when
  `DATABASE_URL` is set and skip (with a logged reason) when it is not — for
  `fdb-realtime/tests/listen_live_pg.rs`, `fdb-reflection/tests/pgvector_rpc.rs`,
  `fdb-app/tests/meta_listener.rs`, `fdb-gateway/tests/a2ui_{schema,trigger}_test.rs`.
  (A small shared `require_db!()`-style helper or a `DATABASE_URL`-guarded early return.)
- **Add missing DB-backed tests:**
  - Embedding REST path: `GET /<schema>/<table>?select=*,child(*)` returns correctly
    nested JSON (parent row + embedded child array/object) under RLS — against a real
    schema with an FK, proving `embed_schema_from_model` + `build_inner_query` +
    `PgRest`/reflection execution end-to-end.
  - `PgRest::execute`: a filtered list query returns the expected rows under a real
    `RlsContext` (6-GUC `SET LOCAL`), proving the fdb-query → SQL → rows path.
- Keep the tests self-contained (own ephemeral schema/table, cleanup) and serialized on
  shared DDL (advisory lock, per the pattern established in `listen_live_pg.rs`).

## Out of Scope
- The CI service itself (c003). New product features (none — tests only, plus any test
  helper).

## Acceptance Criteria
- [ ] The listed tests run (not skip) when `DATABASE_URL` is set; skip cleanly when unset.
- [ ] New embedding-REST and `PgRest::execute` DB tests exist and pass against the CI DB.
- [ ] `cargo test --workspace` with `DATABASE_URL` set is green in the c003 DB stage.
- [ ] Default `cargo test` (no `DATABASE_URL`) still passes (DB tests skip, not fail).
