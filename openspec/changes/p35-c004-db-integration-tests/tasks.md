# Tasks — p35-c004-db-integration-tests

- [x] Remove #[ignore] from listen_live_pg (2) + pgvector_rpc (3) → uniform DATABASE_URL gating.
- [x] Add pgrest_live_pg.rs: PgRest::execute filtered read under real RLS acquire (verified LIVE).
- [x] Add embedding_live_pg.rs: embedding projection SQL → nested JSON on real FK schema (verified LIVE).
- [x] FIX (critical, found live): PgBackend::acquire used SET ... = $1 (SET rejects binds);
      switched GUCs to set_config($1) + validated-identifier SET LOCAL ROLE. Whole RLS path was broken.
- [x] Verify: both new tests pass against local Postgres; default cargo test DB-free green; ci-check green.
