# Goals — p3.5-ci-postgres-hardening

**Phase gate:** The real-time and REST paths proven manually against a live Postgres
in p3 become **CI-gating**, and the pre-existing `fdb-gateway` test debt is cleared —
so `cargo test --workspace` is green and meaningful with a database in CI.

Seeded from p3-auth-rls-keto `reflection.md` (recommended next-phase focus #1 + #3)
and `handoffs/reflect.md`. Closes the "proven only on manual `--ignored` runs" gap.

---

## Goals

- **G1** — CI Postgres service: provision a PG18 + pgvector + pg_graphql database in the
  CI pipeline (`scripts/ci-check.sh` / Dagger), export `DATABASE_URL`, so DB-backed tests
  run in CI. Resolves **OQ-9**.
- **G2** — Un-ignore the live-PG tests: remove `#[ignore]` (or gate on `DATABASE_URL`
  presence rather than the ignore attribute) for `fdb-realtime/tests/listen_live_pg.rs`,
  the `fdb-reflection` pgvector/meta-listener tests, and add a DB-backed test for the
  embedding REST path (`select=*,child(*)` → correct nested JSON) and `PgRest::execute`.
- **G3** — Fix pre-existing `fdb-gateway` test debt: isolate the `keto_sync` env-var test
  (`keto_sync_config_ignores_non_numeric_env`) so it no longer flakes under parallel
  `set_var`, and clear the `uninlined_format_args` lint in `tests/a2ui_seed_test.rs`.
- **G4** — Workspace clippy gate is clean end-to-end: `cargo clippy --workspace
  --all-targets -- -D warnings` passes (currently blocked by the `hello-component` example
  crate's macro-generated `used_underscore_items` lint — allow/annotate it narrowly).
- **G5** — Reconcile p3 phase bookkeeping carried forward: c019 (PostgREST engine) and
  c020 (LISTEN source) are recorded as delivered; c017 marked superseded-by-c020; c018
  resolved/re-scoped against the merged introspection work.

## Dependencies from p3-auth-rls-keto

- `fdb-query`, `PgRest::execute`, resource-embedding wiring — delivered (c019, on main).
- `ListenChangeSource` + migration 0006 + `#[ignore]`d live-PG tests — delivered (c020, on main).
- The GraphQL subscription seam + RLS re-query — delivered (G4/G5, on main).

## Open questions carried in

- **OQ-9** — pgvector ≥ 0.7.0 + pg_graphql in the CI PG18 image; wiring `DATABASE_URL`
  in CI is the core of G1/G2.
- **OQ-FRF-1** — unchanged; the FRF `WatchEntityType` path stays deferred. `ListenChangeSource`
  is the working backend this phase hardens.
