# Tasks — p35-c003-ci-postgres-service

- [x] docker/postgres/Dockerfile: PG18 + pgvector (v0.8.5) + pg_graphql (v1.6.1), pinned.
      (Originally pinned to v0.8.0 / v1.5.11 — bumped 2026-07-14, GH issue #7: v0.8.0 does
      not compile against PG18's changed `vacuum_delay_point()` signature, and v1.5.11 has
      no pg18 release asset.)
- [x] scripts/ci-test.sh: unit stage (always) + db-integration stage (DATABASE_URL-gated,
      migrate + cargo test --include-ignored). Verified: shellcheck clean, no-DB path runs green.
- [x] .dagger/main.go: CheckDb — pinned PG service binding + DATABASE_URL + ci-test.sh (gofmt clean).
- [x] Build docker/postgres image end-to-end and verify both extensions load on PG18 —
      done on a host with Docker (2026-07-14): `docker build docker/postgres` succeeds,
      and `CREATE EXTENSION vector; CREATE EXTENSION pg_graphql;` both succeed at runtime
      (versions 0.8.5 / 1.6.1). The full `dagger`-driven CheckDb pipeline itself remains
      unverified in this env (no dagger CLI) — only the underlying image build was tested.
- [~] flint_meta bootstrap for the full migration set — surfaced as a follow-up (migrations
      assume the ext-flint-meta pgrx schema exists); wire before CheckDb can be fully green.
