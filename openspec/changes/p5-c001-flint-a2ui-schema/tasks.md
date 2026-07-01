# p5-c001 Tasks — flint_a2ui Schema

## Tasks

- [x] Add `CREATE EXTENSION IF NOT EXISTS vector;` to `images/postgres18/Dockerfile` and verify pgvector installs cleanly on PG18 (OQ-9) — pgvector already in Dockerfile; CREATE EXTENSION added to migration SQL
- [x] Create `migrations/0002_flint_a2ui.sql` with all tables in dependency order (applications → components → embeddings → schemas → bindings → events → assembly_rules → roles → role_assignments)
- [x] Add HNSW index on `flint_a2ui.embeddings` with `m=16, ef_construction=64, vector_cosine_ops`
- [x] Add `flint_a2ui.semantic_search()` stub function
- [x] Enable RLS on `flint_a2ui.components` with `component_access` policy
- [x] Enable RLS on `flint_a2ui.events` with insert-only policy (deny UPDATE/DELETE)
- [x] Add `is_system` column to `flint_a2ui.applications` and seed `flint-admin` + `flint-playground`
- [x] Wire migration into `fdb-gateway` startup (apply `0002_flint_a2ui.sql` via sqlx migrate) — added `sqlx::migrate!("../../migrations")` to main.rs; added `migrate` feature to workspace sqlx
- [x] Write gate test: schema tables exist, pgvector extension present, HNSW index exists, RLS active, `semantic_search()` callable — crates/fdb-gateway/tests/a2ui_schema_test.rs
- [x] Verify `current_setting('app.jwt_claims', true)` returns NULL (not error) when GUC is not set — gate test `test_jwt_claims_guc_returns_null_when_unset` covers this
